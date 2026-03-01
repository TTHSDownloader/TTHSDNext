use std::sync::Arc;
use std::str::FromStr;
use tokio::sync::RwLock;

use super::downloader_interface::{Downloader, BaseDownloader};
use super::downloader::{DownloadTask, DownloadConfig};
use super::performance_monitor::PerformanceMonitor;

/// Metalink 下载器
/// 支持 Metalink 4.0 (.metalink / .meta4) 格式
/// 解析 XML 文件提取镜像 URL 列表，选择最优镜像用 HTTP 下载
pub struct MetalinkDownloader {
    base: BaseDownloader,
    monitor: Option<Arc<PerformanceMonitor>>,
}

impl MetalinkDownloader {
    pub async fn new(config: Arc<RwLock<DownloadConfig>>) -> Self {
        let monitor = super::performance_monitor::get_global_monitor().await;
        MetalinkDownloader {
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
impl Downloader for MetalinkDownloader {
    async fn download(&mut self, task: &DownloadTask) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = &task.url;
        let save_path = task.save_path.clone();

        eprintln!("Metalink 下载: {}", url);

        // 1. 获取 .metalink / .meta4 文件内容
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("HTTP client 创建失败: {}", e))?;

        let xml_text = client.get(url)
            .send().await
            .map_err(|e| format!("获取 Metalink 文件失败: {}", e))?
            .text().await
            .map_err(|e| format!("读取 Metalink 内容失败: {}", e))?;

        // 2. 解析 Metalink XML
        let metalink = metalink::Metalink4::from_str(&xml_text)
            .map_err(|e| format!("解析 Metalink 失败: {}", e))?;

        if metalink.files.is_empty() {
            return Err("Metalink 文件中没有找到任何文件条目".into());
        }

        // 3. 取第一个文件条目（通常只有一个）
        let file_entry = &metalink.files[0];
        let file_name = &file_entry.name;

        eprintln!("Metalink 文件名: {}", file_name);
        if let Some(size) = file_entry.size {
            eprintln!("Metalink 文件大小: {} bytes ({:.2} MB)", size, size as f64 / 1024.0 / 1024.0);
            if let Some(ref monitor) = self.monitor {
                monitor.set_total_bytes(size as i64);
            }
        }

        // 4. 提取所有 HTTP(S) 镜像 URL，按优先级排序
        let mut mirror_urls: Vec<(u32, String)> = file_entry.urls.iter()
            .filter_map(|u| {
                let url_str = u.value.clone();
                if url_str.starts_with("http://") || url_str.starts_with("https://") {
                    // priority 越小越优先（Metalink 规范）
                    Some((u.priority.unwrap_or(999999), url_str))
                } else {
                    None
                }
            })
            .collect();

        mirror_urls.sort_by_key(|(priority, _)| *priority);

        if mirror_urls.is_empty() {
            return Err("Metalink 中没有可用的 HTTP(S) 镜像 URL".into());
        }

        eprintln!("找到 {} 个镜像 URL，使用优先级最高的镜像", mirror_urls.len());
        for (p, u) in &mirror_urls {
            eprintln!("  [优先级={}] {}", p, u);
        }

        // 5. 构建下载任务，使用第一个（最高优先级）URL
        //    实际下载委托给 HTTP 下载器
        let best_url = mirror_urls[0].1.clone();
        eprintln!("选择镜像: {}", best_url);

        // 直接用 reqwest 流式下载（避免循环依赖 HTTPDownloader）
        let response = client.get(&best_url)
            .send().await
            .map_err(|e| format!("Metalink HTTP 请求失败: {}", e))?;

        let total = response.content_length().unwrap_or(0) as i64;
        if total > 0 {
            if let Some(ref monitor) = self.monitor {
                monitor.set_total_bytes(total);
            }
        }

        let mut file = tokio::fs::File::create(&save_path).await
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

        eprintln!("Metalink 下载完成: {:.2} MB", downloaded as f64 / 1024.0 / 1024.0);
        Ok(())
    }

    fn get_type(&self) -> String {
        "Metalink".to_string()
    }

    async fn cancel(&mut self, _downloader: Box<dyn Downloader>) {
        self.base.running = false;
    }

    async fn get_snapshot(&self) -> Option<Box<dyn std::any::Any>> {
        None
    }
}

impl Default for MetalinkDownloader {
    fn default() -> Self {
        MetalinkDownloader {
            base: BaseDownloader::new(),
            monitor: None,
        }
    }
}
