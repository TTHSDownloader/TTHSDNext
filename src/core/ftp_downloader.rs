use std::sync::Arc;
use std::time::Instant;
use std::io::Write;
use tokio::sync::RwLock;

use super::downloader_interface::{Downloader, BaseDownloader};
use super::downloader::{DownloadTask, DownloadConfig};
use super::performance_monitor::PerformanceMonitor;

/// FTP 下载器
/// 使用 suppaftp 的同步 API + tokio::task::spawn_blocking
/// suppaftp 的 async tokio API 有复杂的泛型推断问题，同步 API 更稳定
pub struct FTPDownloader {
    base: BaseDownloader,
    monitor: Option<Arc<PerformanceMonitor>>,
}

impl FTPDownloader {
    pub async fn new(config: Arc<RwLock<DownloadConfig>>) -> Self {
        let monitor = super::performance_monitor::get_global_monitor().await;

        FTPDownloader {
            base: BaseDownloader {
                config: Some(config),
                running: true,
                ..Default::default()
            },
            monitor,
        }
    }

    /// 解析 FTP URL 为 (host:port, path, username, password)
    fn parse_ftp_url(url: &str) -> Result<(String, String, String, String), Box<dyn std::error::Error + Send + Sync>> {
        let parsed = url::Url::parse(url)
            .map_err(|e| format!("无效的 FTP URL: {}", e))?;

        let host = parsed.host_str()
            .ok_or("FTP URL 缺少主机名")?
            .to_string();
        let port = parsed.port().unwrap_or(21);
        let path = parsed.path().to_string();
        let username = if parsed.username().is_empty() {
            "anonymous".to_string()
        } else {
            parsed.username().to_string()
        };
        let password = parsed.password().unwrap_or("anonymous@").to_string();

        Ok((format!("{}:{}", host, port), path, username, password))
    }
}

#[async_trait::async_trait]
impl Downloader for FTPDownloader {
    async fn download(&mut self, task: &DownloadTask) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (addr, path, username, password) = Self::parse_ftp_url(&task.url)?;
        let save_path = task.save_path.clone();
        let monitor = self.monitor.clone();

        eprintln!("FTP 连接: {} (用户: {})", addr, username);

        // 在阻塞线程中执行同步 FTP 操作
        let result = tokio::task::spawn_blocking(move || -> Result<(i64, f64), String> {
            use suppaftp::FtpStream;

            // 建立连接
            let mut ftp = FtpStream::connect(&addr)
                .map_err(|e| format!("FTP 连接失败: {}", e))?;

            // 登录
            ftp.login(&username, &password)
                .map_err(|e| format!("FTP 登录失败: {}", e))?;

            // 设置二进制传输模式
            ftp.transfer_type(suppaftp::types::FileType::Binary)
                .map_err(|e| format!("设置二进制模式失败: {}", e))?;

            // 获取文件大小
            let file_size = ftp.size(&path)
                .map_err(|e| format!("获取文件大小失败: {}", e))? as i64;

            eprintln!("FTP 文件大小: {} bytes ({:.2} MB)",
                file_size, file_size as f64 / 1024.0 / 1024.0);

            // 创建输出文件
            let mut file = std::fs::File::create(&save_path)
                .map_err(|e| format!("创建文件失败: {}", e))?;

            // 使用 retr 回调进行流式下载
            let start_time = Instant::now();
            let downloaded: i64 = ftp.retr(&path, |reader| {
                let mut buf = vec![0u8; 64 * 1024]; // 64KB buffer
                let mut total: i64 = 0;

                loop {
                    let n = reader.read(&mut buf)
                        .map_err(|e| suppaftp::FtpError::ConnectionError(e))?;
                    if n == 0 {
                        break;
                    }

                    file.write_all(&buf[..n])
                        .map_err(|e| suppaftp::FtpError::ConnectionError(e))?;

                    total += n as i64;
                }

                Ok(total)
            }).map_err(|e| format!("FTP 下载失败: {}", e))?;

            let elapsed = start_time.elapsed().as_secs_f64();

            // 断开连接
            let _ = ftp.quit();

            // 验证大小
            if downloaded != file_size {
                return Err(format!("FTP 下载不完整: {}/{} bytes", downloaded, file_size));
            }

            Ok((downloaded, elapsed))
        }).await.map_err(|e| format!("FTP 下载线程异常: {}", e))?;

        match result {
            Ok((downloaded, elapsed)) => {
                // 更新进度监控
                if let Some(ref monitor) = monitor {
                    monitor.set_total_bytes(downloaded);
                    monitor.add_bytes(downloaded).await;
                }

                let speed_mbps = if elapsed > 0.0 {
                    (downloaded as f64 / 1024.0 / 1024.0) / elapsed
                } else { 0.0 };

                eprintln!("FTP 下载完成: {:.2} MB, 用时 {:.1}s, 速度 {:.2} MB/s",
                    downloaded as f64 / 1024.0 / 1024.0, elapsed, speed_mbps);

                Ok(())
            }
            Err(e) => Err(e.into())
        }
    }

    fn get_type(&self) -> String {
        "FTP".to_string()
    }

    async fn cancel(&mut self, _downloader: Box<dyn Downloader>) {
        self.base.running = false;
    }

    async fn get_snapshot(&self) -> Option<Box<dyn std::any::Any>> {
        None
    }
}

impl Default for FTPDownloader {
    fn default() -> Self {
        FTPDownloader {
            base: BaseDownloader::new(),
            monitor: None,
        }
    }
}
