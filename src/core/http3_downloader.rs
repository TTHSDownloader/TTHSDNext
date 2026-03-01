use std::sync::Arc;
use std::net::SocketAddr;
use tokio::sync::RwLock;
use bytes::Buf;

use super::downloader_interface::{Downloader, BaseDownloader};
use super::downloader::{DownloadTask, DownloadConfig};
use super::performance_monitor::PerformanceMonitor;

/// HTTP/3 下载器
/// 使用 QUIC (quinn) + HTTP/3 (h3) 进行下载
/// 失败时上层 get_downloader 可回退到 HTTPDownloader
pub struct HTTP3Downloader {
    base: BaseDownloader,
    monitor: Option<Arc<PerformanceMonitor>>,
}

impl HTTP3Downloader {
    pub async fn new(config: Arc<RwLock<DownloadConfig>>) -> Self {
        let monitor = super::performance_monitor::get_global_monitor().await;
        HTTP3Downloader {
            base: BaseDownloader {
                config: Some(config),
                running: true,
                ..Default::default()
            },
            monitor,
        }
    }

    /// 构建 rustls TLS 配置（用于 QUIC）
    fn build_tls_config() -> Result<Arc<rustls::ClientConfig>, Box<dyn std::error::Error + Send + Sync>> {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        Ok(Arc::new(tls_config))
    }

    /// 构建 QUIC 端点
    fn build_quic_endpoint() -> Result<quinn::Endpoint, Box<dyn std::error::Error + Send + Sync>> {
        let tls_config = Self::build_tls_config()?;

        let quic_client_config = quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(tls_config.as_ref().clone())
                .map_err(|e| format!("QUIC TLS 配置失败: {}", e))?
        ));

        let bind_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let mut endpoint = quinn::Endpoint::client(bind_addr)
            .map_err(|e| format!("QUIC Endpoint 创建失败: {}", e))?;

        endpoint.set_default_client_config(quic_client_config);
        Ok(endpoint)
    }
}

#[async_trait::async_trait]
impl Downloader for HTTP3Downloader {
    async fn download(&mut self, task: &DownloadTask) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url_str = &task.url;

        // 解析 URL
        let url = url::Url::parse(url_str)
            .map_err(|e| format!("URL 解析失败: {}", e))?;

        let host = url.host_str()
            .ok_or("URL 缺少主机名")?
            .to_string();
        let port = url.port().unwrap_or(443);
        let path = url.path().to_string();
        let query = url.query().map(|q| format!("?{}", q)).unwrap_or_default();
        let full_path = format!("{}{}", path, query);

        eprintln!("HTTP/3 下载: {}:{}{}", host, port, full_path);

        // 构建 QUIC 端点
        let endpoint = Self::build_quic_endpoint()
            .map_err(|e| format!("QUIC 端点构建失败: {}", e))?;

        // DNS 解析
        let addr_str = format!("{}:{}", host, port);
        let addrs: Vec<SocketAddr> = tokio::net::lookup_host(&addr_str).await
            .map_err(|e| format!("DNS 解析失败 ({}): {}", addr_str, e))?
            .collect();

        let server_addr = addrs.into_iter().next()
            .ok_or_else(|| format!("无法解析主机: {}", host))?;

        // 建立 QUIC 连接
        let connecting = endpoint.connect(server_addr, &host)
            .map_err(|e| format!("QUIC 连接发起失败: {}", e))?;

        let quic_conn = connecting.await
            .map_err(|e| format!("QUIC 握手失败: {}", e))?;

        eprintln!("HTTP/3 QUIC 握手成功 ({})", server_addr);

        // 建立 h3 连接
        let (mut driver, mut send_request) = h3::client::new(h3_quinn::Connection::new(quic_conn))
            .await
            .map_err(|e| format!("HTTP/3 连接建立失败: {}", e))?;

        // 在后台驱动连接
        let _driver_task = tokio::spawn(async move {
            let _ = futures::future::poll_fn(|cx| driver.poll_close(cx)).await;
        });

        // 构建 HTTP/3 GET 请求
        let request = http::Request::builder()
            .method(http::Method::GET)
            .uri(url_str.as_str())
            .header("host", &host)
            .header("user-agent", "TTHSDNext/1.0 (HTTP/3)")
            .header("accept", "*/*")
            .body(())
            .map_err(|e| format!("HTTP/3 请求构建失败: {}", e))?;

        let mut stream = send_request.send_request(request).await
            .map_err(|e| format!("HTTP/3 发送请求失败: {}", e))?;

        stream.finish().await
            .map_err(|e| format!("HTTP/3 流结束失败: {}", e))?;

        // 读取响应头
        let response = stream.recv_response().await
            .map_err(|e| format!("HTTP/3 接收响应失败: {}", e))?;

        let status = response.status();
        eprintln!("HTTP/3 响应状态: {}", status);

        if !status.is_success() {
            return Err(format!("HTTP/3 服务器返回错误: {}", status).into());
        }

        // 从响应头获取 Content-Length
        let total = response.headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        if total > 0 {
            if let Some(ref monitor) = self.monitor {
                monitor.set_total_bytes(total);
            }
        }

        // 创建输出文件
        let mut file = tokio::fs::File::create(&task.save_path).await
            .map_err(|e| format!("创建文件失败: {}", e))?;

        // 流式读取响应体
        let mut downloaded: i64 = 0;
        use tokio::io::AsyncWriteExt;

        loop {
            match stream.recv_data().await {
                Ok(Some(mut data)) => {
                    // data implements bytes::Buf
                    use tokio::io::AsyncWriteExt;
                    while data.has_remaining() {
                        let chunk_len = data.remaining().min(65536);
                        let chunk = data.chunk()[..chunk_len].to_vec();
                        file.write_all(&chunk).await
                            .map_err(|e| format!("写入文件失败: {}", e))?;
                        data.advance(chunk_len);
                        downloaded += chunk_len as i64;
                        if let Some(ref monitor) = self.monitor {
                            monitor.add_bytes(chunk_len as i64).await;
                        }
                    }
                }
                Ok(None) => break, // 响应体结束
                Err(e) => return Err(format!("HTTP/3 数据读取失败: {}", e).into()),
            }
        }

        eprintln!("HTTP/3 下载完成: {:.2} MB", downloaded as f64 / 1024.0 / 1024.0);
        Ok(())
    }

    fn get_type(&self) -> String {
        "HTTP/3".to_string()
    }

    async fn cancel(&mut self, _downloader: Box<dyn Downloader>) {
        self.base.running = false;
    }

    async fn get_snapshot(&self) -> Option<Box<dyn std::any::Any>> {
        None
    }
}

impl Default for HTTP3Downloader {
    fn default() -> Self {
        HTTP3Downloader {
            base: BaseDownloader::new(),
            monitor: None,
        }
    }
}
