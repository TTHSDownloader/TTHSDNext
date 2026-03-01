use std::sync::Arc;
use tokio::sync::RwLock;

use super::downloader_interface::{Downloader, BaseDownloader};
use super::downloader::{DownloadTask, DownloadConfig};
use super::performance_monitor::PerformanceMonitor;

/// ED2K 下载器
///
/// 解析 ed2k://|file|<name>|<size>|<hash>|/ 格式的链接
/// 通过公共 HTTP 网关将 ED2K 转为 HTTP 下载
/// 使用网关: https://ed2k.lyoko.io/hash/<hash>
pub struct ED2KDownloader {
    base: BaseDownloader,
    monitor: Option<Arc<PerformanceMonitor>>,
}

/// 解析后的 ED2K 链接信息
struct Ed2kInfo {
    name: String,
    size: u64,
    hash: String,
}

impl ED2KDownloader {
    pub async fn new(config: Arc<RwLock<DownloadConfig>>) -> Self {
        let monitor = super::performance_monitor::get_global_monitor().await;
        ED2KDownloader {
            base: BaseDownloader {
                config: Some(config),
                running: true,
                ..Default::default()
            },
            monitor,
        }
    }

    /// 解析 ed2k:// URL
    /// 格式: ed2k://|file|<filename>|<filesize>|<md4hash>|/
    fn parse_ed2k_url(url: &str) -> Result<Ed2kInfo, String> {
        // 去掉 ed2k:// 前缀
        let stripped = url.strip_prefix("ed2k://")
            .ok_or("不是有效的 ed2k:// URL")?;

        // 按 | 分割: ["", "file", name, size, hash, "", ""]
        let parts: Vec<&str> = stripped.split('|').collect();

        if parts.len() < 5 {
            return Err(format!("ED2K URL 格式错误，分段数量不足: {}", url));
        }

        // 检查类型（目前只支持 file）
        if parts[1] != "file" {
            return Err(format!("不支持的 ED2K 类型: '{}' (仅支持 file)", parts[1]));
        }

        let name = url::form_urlencoded::parse(parts[2].as_bytes())
            .next()
            .map(|(k, _)| k.into_owned())
            .unwrap_or_else(|| parts[2].to_string());
        // 简单的 URL decode（文件名可能被 % 编码）
        let name = percent_decode(parts[2]);

        let size: u64 = parts[3].parse()
            .map_err(|_| format!("ED2K 文件大小解析失败: '{}'", parts[3]))?;

        let hash = parts[4].to_string();
        if hash.len() != 32 {
            return Err(format!("ED2K hash 长度不正确: {} (应为 32)", hash.len()));
        }

        Ok(Ed2kInfo { name, size, hash })
    }
}

/// 简单的 URL percent-decode（仅处理 %XX 序列）
fn percent_decode(s: &str) -> String {
    let mut result = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i+1..i+3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    result.push(byte as char);
                    i += 3;
                    continue;
                }
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

#[async_trait::async_trait]
impl Downloader for ED2KDownloader {
    async fn download(&mut self, task: &DownloadTask) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ed2k_info = Self::parse_ed2k_url(&task.url)
            .map_err(|e| format!("ED2K URL 解析失败: {}", e))?;

        eprintln!("ED2K 下载: {} ({} bytes, hash={})",
            ed2k_info.name, ed2k_info.size, ed2k_info.hash);

        if let Some(ref monitor) = self.monitor {
            monitor.set_total_bytes(ed2k_info.size as i64);
        }

        // 构建 HTTP 网关 URL（lyoko.io ED2K 网关）
        let gateway_url = format!("https://ed2k.lyoko.io/hash/{}", ed2k_info.hash);
        eprintln!("通过 HTTP 网关下载: {}", gateway_url);

        // 用 reqwest 流式下载
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (compatible; TTHSDNext)")
            .build()
            .map_err(|e| format!("HTTP client 创建失败: {}", e))?;

        // 尝试网关请求，失败时返回友好错误
        let response = client.get(&gateway_url)
            .send().await
            .map_err(|e| format!("ED2K 网关请求失败 ({}): {}", gateway_url, e))?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!(
                "ED2K 网关返回错误 HTTP {}: {}\n  hash={}\n  网关={}",
                status.as_u16(), status.canonical_reason().unwrap_or("Unknown"),
                ed2k_info.hash, gateway_url
            ).into());
        }

        let total = response.content_length().unwrap_or(ed2k_info.size) as i64;
        if let Some(ref monitor) = self.monitor {
            monitor.set_total_bytes(total);
        }

        let mut file = tokio::fs::File::create(&task.save_path).await
            .map_err(|e| format!("创建文件失败: {}", e))?;

        let mut stream = response.bytes_stream();
        let mut downloaded: i64 = 0;

        use futures::StreamExt;
        use tokio::io::AsyncWriteExt;

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| format!("流读取失败: {}", e))?;
            file.write_all(&bytes).await
                .map_err(|e| format!("写入失败: {}", e))?;
            downloaded += bytes.len() as i64;
            if let Some(ref monitor) = self.monitor {
                monitor.add_bytes(bytes.len() as i64).await;
            }
        }

        eprintln!("ED2K 下载完成: {:.2} MB ({})",
            downloaded as f64 / 1024.0 / 1024.0, ed2k_info.name);
        Ok(())
    }

    fn get_type(&self) -> String {
        "ED2K".to_string()
    }

    async fn cancel(&mut self, _downloader: Box<dyn Downloader>) {
        self.base.running = false;
    }

    async fn get_snapshot(&self) -> Option<Box<dyn std::any::Any>> {
        None
    }
}

impl Default for ED2KDownloader {
    fn default() -> Self {
        ED2KDownloader {
            base: BaseDownloader::new(),
            monitor: None,
        }
    }
}
