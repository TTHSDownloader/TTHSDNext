use std::sync::Arc;
use tokio::sync::RwLock;
use super::downloader::DownloadConfig;
use super::downloader_interface::Downloader;
use super::http_downloader::HTTPDownloader;

#[cfg(feature = "ftp")]
use super::ftp_downloader::FTPDownloader;
#[cfg(feature = "torrent")]
use super::torrent_downloader::TorrentDownloader;
#[cfg(feature = "metalink")]
use super::metalink_downloader::MetalinkDownloader;
#[cfg(feature = "ed2k")]
use super::ed2k_downloader::ED2KDownloader;
#[cfg(feature = "http3")]
use super::http3_downloader::HTTP3Downloader;
#[cfg(feature = "sftp")]
use super::sftp_downloader::SFTPDownloader;

/// Downloader factory function
/// Automatically routes to the appropriate downloader implementation based on URL scheme
/// All downloaders implement the `Downloader` trait, callers don't need to know the concrete type
/// Currently supported protocols:
/// - `http://`, `https://` -> HTTPDownloader
///
/// Planned support:
/// - `ftp://`, `ftps://`   -> FTPDownloader
/// - `sftp://`             -> SFTPDownloader
/// - `magnet:?`            -> TorrentDownloader (BT/DHT/Magnet)
/// - `ed2k://`             -> ED2KDownloader
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
        Protocol::Http => {
            #[cfg(feature = "http3")]
            {
                if probe_h3_support(&url).await {
                    eprintln!("Server supports HTTP/3, using QUIC download");
                    return Box::new(HTTP3Downloader::new(config).await) as Box<dyn Downloader>;
                }
            }
            Box::new(HTTPDownloader::new(config).await) as Box<dyn Downloader>
        }
        #[cfg(feature = "ftp")]
        Protocol::Ftp => Box::new(FTPDownloader::new(config).await),
        #[cfg(feature = "torrent")]
        Protocol::BitTorrent => Box::new(TorrentDownloader::new(config).await),
        #[cfg(feature = "ed2k")]
        Protocol::Ed2k => Box::new(ED2KDownloader::new(config).await),
        #[cfg(feature = "metalink")]
        Protocol::Metalink => Box::new(MetalinkDownloader::new(config).await),
        #[cfg(feature = "sftp")]
        Protocol::Sftp => Box::new(SFTPDownloader::new(config).await),
        _ => {
            eprintln!("Warning: Unknown protocol '{}', falling back to HTTP download", url.split("://").next().unwrap_or("unknown"));
            Box::new(HTTPDownloader::new(config).await)
        }
    }
}

/// Send HEAD request, check if Alt-Svc header contains h3
/// Timeout 800ms, return false on failure (non-blocking)
#[cfg(feature = "http3")]
async fn probe_h3_support(url: &str) -> bool {
    use std::time::Duration;

    // Reuse global HTTP client (if available), otherwise create temporary
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(800))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    match client.head(url).send().await {
        Ok(resp) => {
            // Check Alt-Svc header: h3="..." or h3-29="..."
            resp.headers()
                .get("alt-svc")
                .and_then(|v| v.to_str().ok())
                .map(|s| {
                    let lower = s.to_lowercase();
                    lower.contains("h3=") || lower.contains("h3-")
                })
                .unwrap_or(false)
        }
        Err(_) => false,
    }
}

/// Supported download protocol enum
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

/// Detect protocol type from URL string
/// File extensions take precedence over HTTP scheme so that
/// `.metalink`/`.meta4`/`.torrent` URLs are routed to the correct handler
/// even when served over HTTP(S).
fn detect_scheme(url: &str) -> Protocol {
    let lower = url.to_lowercase();
    // Strip query string before checking file extensions
    let path = lower.split('?').next().unwrap_or(&lower);
    if path.ends_with(".metalink") || path.ends_with(".meta4") {
        return Protocol::Metalink;
    }
    if path.ends_with(".torrent") {
        return Protocol::BitTorrent;
    }
    // Scheme-based checks
    if lower.starts_with("http://") || lower.starts_with("https://") {
        Protocol::Http
    } else if lower.starts_with("ftp://") || lower.starts_with("ftps://") {
        Protocol::Ftp
    } else if lower.starts_with("sftp://") {
        Protocol::Sftp
    } else if lower.starts_with("magnet:") {
        Protocol::BitTorrent
    } else if lower.starts_with("ed2k://") {
        Protocol::Ed2k
    } else {
        Protocol::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── HTTP ──

    #[test]
    fn test_detect_http() {
        assert_eq!(detect_scheme("http://example.com/file.zip"), Protocol::Http);
    }

    #[test]
    fn test_detect_https() {
        assert_eq!(detect_scheme("https://example.com/file.zip"), Protocol::Http);
    }

    #[test]
    fn test_detect_https_with_query() {
        assert_eq!(detect_scheme("https://cdn.example.com/dl?file=123&token=abc"), Protocol::Http);
    }

    #[test]
    fn test_detect_https_mixed_case() {
        assert_eq!(detect_scheme("HTTPS://EXAMPLE.COM/FILE.ZIP"), Protocol::Http);
    }

    // ── FTP ──

    #[test]
    fn test_detect_ftp() {
        assert_eq!(detect_scheme("ftp://ftp.gnu.org/README"), Protocol::Ftp);
    }

    #[test]
    fn test_detect_ftps() {
        assert_eq!(detect_scheme("ftps://secure-ftp.example.com/file"), Protocol::Ftp);
    }

    // ── SFTP ──

    #[test]
    fn test_detect_sftp() {
        assert_eq!(detect_scheme("sftp://user@host:22/path/to/file"), Protocol::Sftp);
    }

    // ── BitTorrent ──

    #[test]
    fn test_detect_magnet() {
        assert_eq!(detect_scheme("magnet:?xt=urn:btih:ABC123&dn=test"), Protocol::BitTorrent);
    }

    #[test]
    fn test_detect_torrent_file() {
        assert_eq!(detect_scheme("https://example.com/ubuntu.torrent"), Protocol::BitTorrent);
    }

    #[test]
    fn test_detect_torrent_file_with_query() {
        assert_eq!(detect_scheme("https://example.com/file.torrent?ref=1"), Protocol::BitTorrent);
    }

    // ── ED2K ──

    #[test]
    fn test_detect_ed2k() {
        assert_eq!(detect_scheme("ed2k://|file|test.iso|1073741824|HASH|/"), Protocol::Ed2k);
    }

    // ── Metalink ──

    #[test]
    fn test_detect_metalink() {
        assert_eq!(detect_scheme("https://example.com/arch.metalink"), Protocol::Metalink);
    }

    #[test]
    fn test_detect_meta4() {
        assert_eq!(detect_scheme("https://example.com/arch.meta4"), Protocol::Metalink);
    }

    // ── Unknown ──

    #[test]
    fn test_detect_unknown_scheme() {
        assert_eq!(detect_scheme("gopher://example.com/file"), Protocol::Unknown);
    }

    #[test]
    fn test_detect_empty_string() {
        assert_eq!(detect_scheme(""), Protocol::Unknown);
    }

    #[test]
    fn test_detect_random_text() {
        assert_eq!(detect_scheme("not a url at all"), Protocol::Unknown);
    }

    #[test]
    fn test_detect_ipfs() {
        assert_eq!(detect_scheme("ipfs://QmHash"), Protocol::Unknown);
    }
}