use std::sync::Arc;
use tokio::sync::RwLock;
use super::downloader::DownloadConfig;
use super::downloader_interface::Downloader;
use super::http_downloader::HTTPDownloader;

/// 下载器工厂函数
///
/// 根据配置返回实现了 `Downloader` trait 的下载器实例。
/// 所有下载器均继承自 `BaseDownloader`，前端不需要关心具体的实现类型。
///
/// 目前支持的下载器类型:
/// - `HTTPDownloader`: HTTP/HTTPS 协议下载（默认）
///
/// 后续可根据 URL scheme 或配置参数扩展更多下载器类型，
/// 如 FTP、P2P、磁力链接等。
pub async fn get_downloader(
    config: Arc<RwLock<DownloadConfig>>,
) -> Box<dyn Downloader> {
    // TODO: 未来可根据 config 中的 URL scheme 或其他字段
    // 自动选择合适的下载器类型，例如:
    //   - "ftp://"  -> FTPDownloader
    //   - "magnet:" -> TorrentDownloader
    //   - 默认      -> HTTPDownloader
    Box::new(HTTPDownloader::new(config).await)
}
