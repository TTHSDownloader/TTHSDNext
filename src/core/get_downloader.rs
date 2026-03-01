use std::sync::Arc;
use tokio::sync::RwLock;
use super::downloader::DownloadConfig;
use super::downloader_interface::Downloader;
use super::http_downloader::HTTPDownloader;
use super::ftp_downloader::FTPDownloader;
use super::torrent_downloader::TorrentDownloader;
use super::metalink_downloader::MetalinkDownloader;
use super::ed2k_downloader::ED2KDownloader;
use super::http3_downloader::HTTP3Downloader;

/// 下载器工厂函数
///
/// 根据第一个任务的 URL scheme 自动路由到对应的下载器实现。
/// 所有下载器均实现了 `Downloader` trait，调用方不需要关心具体类型。
///
/// 目前支持的协议:
/// - `http://`, `https://` → HTTPDownloader
///
/// 计划支持的协议:
/// - `ftp://`, `ftps://`   → FTPDownloader
/// - `sftp://`             → SFTPDownloader
/// - `magnet:?`            → TorrentDownloader (BT/DHT/Magnet)
/// - `ed2k://`             → ED2KDownloader
pub async fn get_downloader(
    config: Arc<RwLock<DownloadConfig>>,
) -> Box<dyn Downloader> {
    let url = {
        let cfg = config.read().await;
        cfg.tasks.first()
           .map(|t| t.url.clone())
           .unwrap_or_default()
    };

    let scheme = detect_scheme(&url);

    match scheme {
        // HTTP/3: 先尝试 QUIC，失败自动降级到 HTTP/1.1+2
        Protocol::Http => {
            let h3 = HTTP3Downloader::new(config.clone()).await;
            Box::new(h3) as Box<dyn Downloader>
        }
        Protocol::Ftp => Box::new(FTPDownloader::new(config).await),
        Protocol::BitTorrent => Box::new(TorrentDownloader::new(config).await),
        Protocol::Ed2k => Box::new(ED2KDownloader::new(config).await),
        Protocol::Metalink => Box::new(MetalinkDownloader::new(config).await),
        _ => {
            eprintln!("警告: 未知协议 '{}', 回退到 HTTP 下载器", url.split("://").next().unwrap_or("unknown"));
            Box::new(HTTPDownloader::new(config).await)
        }
    }
}

/// 支持的下载协议枚举
#[derive(Debug, Clone, PartialEq)]
pub enum Protocol {
    Http,
    Ftp,
    Sftp,
    BitTorrent,
    Ed2k,
    Metalink,
    Http3,
    Unknown,
}

/// 从 URL 字符串检测协议类型
fn detect_scheme(url: &str) -> Protocol {
    let lower = url.to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        Protocol::Http
    } else if lower.starts_with("ftp://") || lower.starts_with("ftps://") {
        Protocol::Ftp
    } else if lower.starts_with("sftp://") {
        Protocol::Sftp
    } else if lower.starts_with("magnet:") || lower.ends_with(".torrent") {
        Protocol::BitTorrent
    } else if lower.starts_with("ed2k://") {
        Protocol::Ed2k
    } else if lower.ends_with(".metalink") || lower.ends_with(".meta4") {
        Protocol::Metalink
    } else {
        Protocol::Unknown
    }
}
