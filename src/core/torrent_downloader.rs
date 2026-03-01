use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;

use librqbit::{AddTorrent, AddTorrentOptions, Session};

use super::downloader_interface::{Downloader, BaseDownloader};
use super::downloader::{DownloadTask, DownloadConfig};
use super::performance_monitor::PerformanceMonitor;

/// BitTorrent 下载器
/// 支持 magnet: 链接、.torrent 文件 URL、DHT 网络、PEX (Peer Exchange)
/// 基于 librqbit — 纯 Rust BitTorrent 客户端库
pub struct TorrentDownloader {
    base: BaseDownloader,
    monitor: Option<Arc<PerformanceMonitor>>,
}

impl TorrentDownloader {
    pub async fn new(config: Arc<RwLock<DownloadConfig>>) -> Self {
        let monitor = super::performance_monitor::get_global_monitor().await;

        TorrentDownloader {
            base: BaseDownloader {
                config: Some(config),
                running: true,
                ..Default::default()
            },
            monitor,
        }
    }
}

#[async_trait::async_trait]
impl Downloader for TorrentDownloader {
    async fn download(&mut self, task: &DownloadTask) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 确定输出目录 (从 save_path 获取父目录)
        let save_path = PathBuf::from(&task.save_path);
        let output_dir = save_path.parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();

        eprintln!("BT 下载: {} -> {:?}", task.url, output_dir);

        // 创建 librqbit Session
        let session = Session::new(output_dir).await
            .map_err(|e| format!("创建 BT Session 失败: {}", e))?;

        // 构建 AddTorrent 参数
        let add_torrent = if task.url.starts_with("magnet:") {
            AddTorrent::from_url(&task.url)
        } else if task.url.ends_with(".torrent") {
            // .torrent 文件 URL — 先下载文件内容
            AddTorrent::from_url(&task.url)
        } else {
            return Err(format!("不支持的 BT URL 格式: {}", task.url).into());
        };

        // 添加种子并开始下载
        let opts = AddTorrentOptions {
            ..Default::default()
        };

        let response = session.add_torrent(add_torrent, Some(opts)).await
            .map_err(|e| format!("添加种子失败: {}", e))?;

        let handle = match response {
            librqbit::AddTorrentResponse::Added(_, handle) => handle,
            librqbit::AddTorrentResponse::AlreadyManaged(_, handle) => {
                eprintln!("BT 种子已存在，继续下载");
                handle
            }
            librqbit::AddTorrentResponse::ListOnly(_info) => {
                eprintln!("BT 种子以只列模式添加");
                return Err("种子以只列模式添加，未开始下载".into());
            }
        };

        eprintln!("BT 下载已开始，等待完成...");

        // 轮询等待下载完成
        let mut last_reported_bytes: u64 = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            let stats = handle.stats();
            let downloaded = stats.progress_bytes;
            let total = stats.total_bytes;

            // 更新进度
            if let Some(ref monitor) = self.monitor {
                let new_bytes = downloaded.saturating_sub(last_reported_bytes);
                if new_bytes > 0 {
                    if last_reported_bytes == 0 {
                        monitor.set_total_bytes(total as i64);
                    }
                    monitor.add_bytes(new_bytes as i64).await;
                    last_reported_bytes = downloaded;
                }
            }

            // 检查是否完成
            if downloaded >= total && total > 0 {
                eprintln!("BT 下载完成: {:.2} MB", total as f64 / 1024.0 / 1024.0);
                break;
            }

            // 检查是否被取消
            if !self.base.running {
                eprintln!("BT 下载被取消");
                return Err("BT 下载被用户取消".into());
            }
        }

        Ok(())
    }

    fn get_type(&self) -> String {
        "BitTorrent".to_string()
    }

    async fn cancel(&mut self, _downloader: Box<dyn Downloader>) {
        self.base.running = false;
    }

    async fn get_snapshot(&self) -> Option<Box<dyn std::any::Any>> {
        None
    }
}

impl Default for TorrentDownloader {
    fn default() -> Self {
        TorrentDownloader {
            base: BaseDownloader::new(),
            monitor: None,
        }
    }
}
