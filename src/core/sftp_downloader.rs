use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use super::downloader_interface::{Downloader, BaseDownloader};
use super::downloader::{DownloadTask, DownloadConfig};
use super::performance_monitor::PerformanceMonitor;

/// SFTP 下载器
/// 使用 russh (纯 Rust SSH) + russh-sftp 实现异步 SFTP 文件下载
pub struct SFTPDownloader {
    base: BaseDownloader,
    monitor: Option<Arc<PerformanceMonitor>>,
}

impl SFTPDownloader {
    pub async fn new(config: Arc<RwLock<DownloadConfig>>) -> Self {
        let monitor = super::performance_monitor::get_global_monitor().await;

        SFTPDownloader {
            base: BaseDownloader {
                config: Some(config),
                running: true,
                ..Default::default()
            },
            monitor,
        }
    }

    /// 解析 SFTP URL 为 (host, port, path, username, password)
    /// 格式: sftp://[user[:password]@]host[:port]/path/to/file
    fn parse_sftp_url(url: &str) -> Result<(String, u16, String, String, String), Box<dyn std::error::Error + Send + Sync>> {
        let parsed = url::Url::parse(url)
            .map_err(|e| format!("无效的 SFTP URL: {}", e))?;

        let host = parsed.host_str()
            .ok_or("SFTP URL 缺少主机名")?
            .to_string();
        let port = parsed.port().unwrap_or(22);
        let path = parsed.path().to_string();
        let username = if parsed.username().is_empty() {
            "root".to_string()
        } else {
            parsed.username().to_string()
        };
        let password = parsed.password().unwrap_or("").to_string();

        if path.is_empty() || path == "/" {
            return Err("SFTP URL 缺少文件路径".into());
        }

        Ok((host, port, path, username, password))
    }
}

/// russh 需要一个 Handler 来处理 SSH 会话事件
/// 这里使用最简实现：接受所有主机密钥，不做额外处理
struct SshHandler;

#[async_trait::async_trait]
impl russh::client::Handler for SshHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // 接受所有主机密钥（下载场景不需要严格验证）
        Ok(true)
    }
}

#[async_trait::async_trait]
impl Downloader for SFTPDownloader {
    async fn download(&mut self, task: &DownloadTask) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (host, port, remote_path, username, password) = Self::parse_sftp_url(&task.url)?;
        let save_path = task.save_path.clone();
        let monitor = self.monitor.clone();

        eprintln!("SFTP 连接: {}@{}:{} 路径: {}", username, host, port, remote_path);

        // 1. 配置 SSH 客户端
        let config = russh::client::Config::default();
        let config = Arc::new(config);

        // 2. 建立 SSH 连接
        let mut session = russh::client::connect(config, (host.as_str(), port), SshHandler)
            .await
            .map_err(|e| format!("SSH 连接失败: {}", e))?;

        // 3. 密码认证
        let auth_result = session.authenticate_password(&username, &password)
            .await
            .map_err(|e| format!("SSH 认证失败: {}", e))?;

        if !auth_result {
            return Err("SSH 密码认证被拒绝".into());
        }

        eprintln!("SSH 认证成功");

        // 4. 打开 SFTP 通道
        let channel = session.channel_open_session()
            .await
            .map_err(|e| format!("打开 SSH 通道失败: {}", e))?;

        channel.request_subsystem(true, "sftp")
            .await
            .map_err(|e| format!("请求 SFTP 子系统失败: {}", e))?;

        let sftp = russh_sftp::client::SftpSession::new(channel.into_stream())
            .await
            .map_err(|e| format!("初始化 SFTP 会话失败: {}", e))?;

        eprintln!("SFTP 会话已建立");

        // 5. 获取远程文件信息
        let metadata = sftp.metadata(&remote_path)
            .await
            .map_err(|e| format!("获取远程文件信息失败: {}", e))?;

        let file_size = metadata.size.unwrap_or(0) as i64;
        eprintln!("SFTP 文件大小: {} bytes ({:.2} MB)",
            file_size, file_size as f64 / 1024.0 / 1024.0);

        // 6. 打开远程文件
        let mut remote_file = sftp.open(&remote_path)
            .await
            .map_err(|e| format!("打开远程文件失败: {}", e))?;

        // 7. 创建本地文件并写入
        let mut local_file = tokio::fs::File::create(&save_path)
            .await
            .map_err(|e| format!("创建本地文件失败: {}", e))?;

        let start_time = Instant::now();
        let mut downloaded: i64 = 0;

        // 流式拷贝
        use tokio::io::AsyncReadExt;
        use tokio::io::AsyncWriteExt;

        let mut buf = vec![0u8; 64 * 1024]; // 64KB buffer
        loop {
            let n = remote_file.read(&mut buf)
                .await
                .map_err(|e| format!("读取远程文件失败: {}", e))?;
            if n == 0 {
                break;
            }

            local_file.write_all(&buf[..n])
                .await
                .map_err(|e| format!("写入本地文件失败: {}", e))?;

            downloaded += n as i64;
        }

        local_file.flush()
            .await
            .map_err(|e| format!("刷新文件缓冲失败: {}", e))?;

        let elapsed = start_time.elapsed().as_secs_f64();

        // 8. 验证大小
        if file_size > 0 && downloaded != file_size {
            return Err(format!("SFTP 下载不完整: {}/{} bytes", downloaded, file_size).into());
        }

        // 9. 更新性能监控
        if let Some(ref monitor) = monitor {
            monitor.set_total_bytes(downloaded);
            monitor.add_bytes(downloaded).await;
        }

        let speed_mbps = if elapsed > 0.0 {
            (downloaded as f64 / 1024.0 / 1024.0) / elapsed
        } else { 0.0 };

        eprintln!("SFTP 下载完成: {:.2} MB, 用时 {:.1}s, 速度 {:.2} MB/s",
            downloaded as f64 / 1024.0 / 1024.0, elapsed, speed_mbps);

        // 10. 关闭 SSH 会话
        let _ = session.disconnect(russh::Disconnect::ByApplication, "", "en")
            .await;

        Ok(())
    }

    fn get_type(&self) -> String {
        "SFTP".to_string()
    }

    async fn cancel(&mut self, _downloader: Box<dyn Downloader>) {
        self.base.running = false;
    }

    async fn get_snapshot(&self) -> Option<Box<dyn std::any::Any>> {
        None
    }
}

impl Default for SFTPDownloader {
    fn default() -> Self {
        SFTPDownloader {
            base: BaseDownloader::new(),
            monitor: None,
        }
    }
}
