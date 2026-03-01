use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::{mpsc, RwLock};
use futures::StreamExt;
use reqwest::{Client, header::{HeaderMap, HeaderValue, RANGE, USER_AGENT, ACCEPT, ACCEPT_LANGUAGE, ACCEPT_ENCODING, CACHE_CONTROL}};
use serde::{Deserialize, Serialize};
use super::downloader_interface::{Downloader, BaseDownloader};
use super::downloader::{DownloadTask, DownloadChunk, DownloadConfig, Event, EventType};
use super::performance_monitor::PerformanceMonitor;
use super::send_message::send_message;

const STALL_TIMEOUT: Duration = Duration::from_secs(30);

/// 全局 HTTP 客户端连接池 (所有 HTTPDownloader 实例共享)
static GLOBAL_HTTP_CLIENT: tokio::sync::OnceCell<Client> = tokio::sync::OnceCell::const_new();

/// 获取全局复用的 HTTP Client (使用 Chrome 133 TLS 指纹伪装)
async fn get_global_client() -> Client {
    GLOBAL_HTTP_CLIENT.get_or_init(|| async {
        use rquest_util::Emulation;

        // 使用 Chrome 133 的 TLS/JA3/HTTP2 指纹，绕过 Cloudflare 等反爬系统
        Client::builder()
            .emulation(Emulation::Chrome133)
            .connect_timeout(Duration::from_secs(15))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(32)
            .tcp_keepalive(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client")
    }).await.clone()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSnapshot {
    #[serde(rename = "downloaded")]
    pub downloaded: i64,
    #[serde(rename = "total_size")]
    pub total_size: i64,
    #[serde(rename = "progress_percentage")]
    pub progress_percentage: f64,
    #[serde(rename = "is_finished")]
    pub is_finished: bool,
    #[serde(rename = "error_message")]
    pub error_message: Option<String>,
    #[serde(rename = "current_speed_bps")]
    pub current_speed_bps: f64,
    #[serde(rename = "average_speed_bps")]
    pub average_speed_bps: f64,
    #[serde(rename = "elapsed_seconds")]
    pub elapsed_seconds: f64,
}

pub struct DownloadStatus {
    total_size: i64,
    downloaded: Arc<RwLock<i64>>,
    error_message: Arc<RwLock<Option<String>>>,
    start_time: Instant,
}

impl DownloadStatus {
    pub fn new(total_size: i64) -> Self {
        DownloadStatus {
            total_size,
            downloaded: Arc::new(RwLock::new(0)),
            error_message: Arc::new(RwLock::new(None)),
            start_time: Instant::now(),
        }
    }

    pub async fn set_error(&self, msg: String) {
        let mut error = self.error_message.write().await;
        *error = Some(msg);
    }

    pub async fn get_error(&self) -> Option<String> {
        let error = self.error_message.read().await;
        error.clone()
    }

    pub async fn add_downloaded(&self, bytes: i64) {
        let mut downloaded = self.downloaded.write().await;
        *downloaded += bytes;
    }

    pub async fn get_downloaded(&self) -> i64 {
        let downloaded = self.downloaded.read().await;
        *downloaded
    }

    pub async fn snapshot(&self, current_speed: f64, average_speed: f64) -> DownloadSnapshot {
        let downloaded = self.get_downloaded().await;
        let error_message = self.get_error().await;

        let progress_percentage = if self.total_size > 0 {
            (downloaded as f64 / self.total_size as f64) * 100.0
        } else {
            0.0
        };

        let is_finished = downloaded >= self.total_size || error_message.is_some();

        DownloadSnapshot {
            downloaded,
            total_size: self.total_size,
            progress_percentage,
            is_finished,
            error_message,
            current_speed_bps: current_speed,
            average_speed_bps: average_speed,
            elapsed_seconds: self.start_time.elapsed().as_secs_f64(),
        }
    }
}

pub struct HTTPDownloader {
    base: BaseDownloader,
    client: Client,
    monitor: Option<Arc<PerformanceMonitor>>,
    status: Option<DownloadStatus>,
}

/// 动态分片工作者 - 跟踪每个分块的实时下载进度
/// progress 和 end_pos 使用 AtomicI64，允许主线程在不加锁的情况下
/// 读取进度并动态修改 end_pos 来切分工作量
struct ChunkWorker {
    /// 该分块的起始偏移量 (固定不变)
    start_pos: i64,
    /// 当前下载进度 (由下载线程原子更新)
    progress: Arc<AtomicI64>,
    /// 该分块的结束偏移量 (可被主线程动态缩减以分配给新 worker)
    end_pos: Arc<AtomicI64>,
}

impl ChunkWorker {
    fn new(start: i64, end: i64) -> Self {
        ChunkWorker {
            start_pos: start,
            progress: Arc::new(AtomicI64::new(start)),
            end_pos: Arc::new(AtomicI64::new(end)),
        }
    }

    /// 剩余未下载的字节数
    fn remaining(&self) -> i64 {
        let end = self.end_pos.load(Ordering::Relaxed);
        let progress = self.progress.load(Ordering::Relaxed);
        (end - progress).max(0)
    }
}

/// 最小可切分大小 (2MB) - 低于此阈值不再切分
const MIN_REASSIGN_SIZE: i64 = 2 * 1024 * 1024;
/// 最大并发连接数上限
const MAX_CONNECTIONS: usize = 64;
impl HTTPDownloader {
    pub async fn new(config: Arc<RwLock<DownloadConfig>>) -> Self {
        let client = get_global_client().await;
        let monitor = super::performance_monitor::get_global_monitor().await;

        HTTPDownloader {
            base: BaseDownloader {
                config: Some(config),
                running: true,
                ..Default::default()
            },
            client,
            monitor,
            status: None,
        }
    }

    async fn get_file_size(&self, url: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client
            .head(url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("HEAD failed: {}", response.status()).into());
        }

        let content_length = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| "Invalid content length")?;

        if content_length <= 0 {
            return Err("Invalid content length".into());
        }

        Ok(content_length)
    }

    fn create_chunks(file_size: i64, chunk_size: i64, thread_count: usize) -> Vec<DownloadChunk> {
        let min_chunks = thread_count * 2;
        let mut chunk_size = chunk_size;

        if file_size / min_chunks as i64 > chunk_size {
            chunk_size = file_size / min_chunks as i64;
            if chunk_size < 1024 * 1024 {
                chunk_size = 1024 * 1024;
            }
        }

        let mut chunks = Vec::new();
        let mut offset = 0;

        while offset < file_size {
            let end = std::cmp::min(offset + chunk_size - 1, file_size - 1);
            chunks.push(DownloadChunk {
                start_offset: offset,
                end_offset: end,
                done: false,
            });
            offset = end + 1;
        }

        chunks
    }

    /// 下载一个分块 (动态版本)
    /// 读取 worker 的 atomic end_pos，这样主线程可以随时缩减我们的工作范围
    async fn download_chunk_dynamic(
        &self,
        task: &DownloadTask,
        start: i64,
        end_pos: Arc<AtomicI64>,
        progress: Arc<AtomicI64>,
        downloaded_size: Arc<RwLock<i64>>,
        _total_size: i64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let current_end = end_pos.load(Ordering::Relaxed);
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"));
        headers.insert(RANGE, HeaderValue::from_str(&format!("bytes={}-{}", start, current_end))?);
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));

        let response = self.client
            .get(&task.url)
            .headers(headers)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Bad status: {}", response.status()).into());
        }

        let last_read = Arc::new(RwLock::new(Instant::now()));
        let stalled_tx = Arc::new(mpsc::channel::<()>(1).0);

        let last_read_clone = last_read.clone();
        let stalled_tx_clone = stalled_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                let elapsed = {
                    let lr = last_read_clone.read().await;
                    lr.elapsed()
                };
                if elapsed > STALL_TIMEOUT {
                    let _ = stalled_tx_clone.send(()).await;
                    break;
                }
            }
        });

        let mut writer = OpenOptions::new()
            .write(true)
            .open(&task.save_path).await?;

        writer.seek(std::io::SeekFrom::Start(start as u64)).await?;

        const BATCH_UPDATE_THRESHOLD: i64 = 512 * 1024;
        let mut local_downloaded = 0i64;
        let mut current_pos = start;

        let mut stream = response.bytes_stream();

        while let Some(bytes_result) = stream.next().await {
            let bytes = bytes_result?;

            {
                let mut lr = last_read.write().await;
                *lr = Instant::now();
            }

            // 检查 end_pos 是否被主线程动态缩减了
            let dynamic_end = end_pos.load(Ordering::Relaxed);
            let bytes_len = bytes.len() as i64;

            if current_pos + bytes_len > dynamic_end + 1 {
                // 只写入到 dynamic_end 为止
                let usable = (dynamic_end + 1 - current_pos).max(0) as usize;
                if usable > 0 {
                    writer.write_all(&bytes[..usable]).await?;
                    local_downloaded += usable as i64;
                    current_pos += usable as i64;
                }
                break; // 主线程已经把我们的范围缩减了，停止下载
            }

            writer.write_all(&bytes).await?;
            local_downloaded += bytes_len;
            current_pos += bytes_len;

            // 更新原子进度
            progress.store(current_pos, Ordering::Relaxed);

            if local_downloaded >= BATCH_UPDATE_THRESHOLD {
                let mut ds = downloaded_size.write().await;
                *ds += local_downloaded;
                drop(ds);

                if let Some(ref monitor) = self.monitor {
                    monitor.add_bytes(local_downloaded).await;
                }

                local_downloaded = 0;
            }

            // 检查是否停滞
            if stalled_tx.try_reserve().is_ok() {
                return Err("connection stalled".into());
            }
        }

        if local_downloaded > 0 {
            let mut ds = downloaded_size.write().await;
            *ds += local_downloaded;
            drop(ds);

            if let Some(ref monitor) = self.monitor {
                monitor.add_bytes(local_downloaded).await;
            }
        }

        // 最终进度更新
        progress.store(current_pos, Ordering::Relaxed);

        Ok(())
    }

    async fn send_error_message(&self, msg: String) {
        if let Some(ref config) = self.base.config {
            let event = Event {
                event_type: EventType::Err,
                name: "Error".to_string(),
                show_name: String::new(),
                id: String::new(),
            };

            let mut data = serde_json::Map::new();
            data.insert("Error".to_string(), serde_json::Value::String(msg));

            let _ = send_message(event, data.into_iter().map(|(k, v)| (k, v)).collect(), config, &self.base.ws_client, &self.base.socket_client).await;
        }
    }
}

impl Default for BaseDownloader {
    fn default() -> Self {
        BaseDownloader {
            total_size: 0,
            downloaded: 0,
            last_downloaded: 0,
            start_time: Instant::now(),
            chunks: Vec::new(),
            ws_client: None,
            socket_client: None,
            config: None,
            running: true,
        }
    }
}

#[async_trait::async_trait]
impl Downloader for HTTPDownloader {
    async fn download(&mut self, task: &DownloadTask) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let file_size = self.get_file_size(&task.url).await?;

        self.status = Some(DownloadStatus::new(file_size));
        
        // 更新全局监控的总大小
        if let Some(ref monitor) = self.monitor {
            monitor.set_total_bytes(file_size);
        }

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&task.save_path).await?;

        // FAT32 文件系统单文件上限为 4GB，超过时给出明确提示
        const FAT32_MAX_FILE_SIZE: i64 = 4_294_967_295; // 4GB - 1 byte
        
        // 尝试预分配文件大小（提升多线程分块写入性能）
        // 如果失败（例如 FAT32 文件系统不支持大文件），则跳过预分配继续下载
        if let Err(e) = file.set_len(file_size as u64).await {
            if file_size > FAT32_MAX_FILE_SIZE {
                return Err(format!(
                    "文件大小 ({:.2} GB) 超过 FAT32 文件系统的 4GB 限制，请将目标路径改为 NTFS/exFAT 分区",
                    file_size as f64 / 1024.0 / 1024.0 / 1024.0
                ).into());
            }
            eprintln!("警告: 无法预分配文件空间 ({}), 将继续下载", e);
        }

        let thread_count = if let Some(ref config) = self.base.config {
            let cfg = config.read().await;
            cfg.thread_count
        } else {
            num_cpus::get() * 2
        };

        let chunk_size = if let Some(ref config) = self.base.config {
            let cfg = config.read().await;
            cfg.chunk_size_mb * 1024 * 1024
        } else {
            10 * 1024 * 1024
        };

        let chunks = Self::create_chunks(file_size, chunk_size as i64, thread_count);
        let downloaded_size = Arc::new(RwLock::new(0i64));

        // 创建动态分片工作者
        let workers: Vec<Arc<ChunkWorker>> = chunks.iter().map(|c| {
            Arc::new(ChunkWorker::new(c.start_offset, c.end_offset))
        }).collect();

        let mut join_set = tokio::task::JoinSet::new();
        let mut active_count = 0usize;

        for worker in &workers {
            let task_clone = task.clone();
            let downloaded_size_clone = downloaded_size.clone();
            let self_clone = self.clone_downloader();
            let start = worker.start_pos;
            let end_pos = worker.end_pos.clone();
            let progress = worker.progress.clone();

            join_set.spawn(async move {
                self_clone.download_chunk_dynamic(
                    &task_clone, start, end_pos, progress,
                    downloaded_size_clone, file_size
                ).await
            });
            active_count += 1;
        }

        // 动态分片: 当一个 worker 完成时，找到剩余最大的 worker 并切分
        while let Some(result) = join_set.join_next().await {
            if let Err(e) = result {
                self.send_error_message(format!("worker error: {:?}", e)).await;
                if let Some(ref status) = self.status {
                    status.set_error(format!("worker error: {:?}", e)).await;
                }
            }

            // 尝试从剩余最大的 worker 切分一半给新 worker
            if active_count < MAX_CONNECTIONS {
                let mut max_remaining = 0i64;
                let mut max_worker: Option<&Arc<ChunkWorker>> = None;

                for w in &workers {
                    let remaining = w.remaining();
                    if remaining > max_remaining {
                        max_remaining = remaining;
                        max_worker = Some(w);
                    }
                }

                if max_remaining > MIN_REASSIGN_SIZE {
                    if let Some(w) = max_worker {
                        let current_progress = w.progress.load(Ordering::Relaxed);
                        let current_end = w.end_pos.load(Ordering::Relaxed);
                        let mid = current_progress + (current_end - current_progress) / 2;

                        // 缩减原 worker 的终点
                        w.end_pos.store(mid, Ordering::Relaxed);

                        // 为新范围创建一个新 worker
                        let new_worker = Arc::new(ChunkWorker::new(mid + 1, current_end));
                        let task_clone = task.clone();
                        let downloaded_size_clone = downloaded_size.clone();
                        let self_clone = self.clone_downloader();
                        let new_start = mid + 1;
                        let new_end_pos = new_worker.end_pos.clone();
                        let new_progress = new_worker.progress.clone();

                        join_set.spawn(async move {
                            self_clone.download_chunk_dynamic(
                                &task_clone, new_start, new_end_pos, new_progress,
                                downloaded_size_clone, file_size
                            ).await
                        });
                        active_count += 1;

                        eprintln!("动态分片: 从 [{}-{}] 切分出 [{}-{}], 当前连接数: {}",
                            current_progress, mid, mid + 1, current_end, active_count);
                    }
                }
            }
        }

        let current_size = *downloaded_size.read().await;
        if current_size != file_size {
            return Err(format!("download incomplete: {}/{} bytes", current_size, file_size).into());
        }

        Ok(())
    }

    fn get_type(&self) -> String {
        "http".to_string()
    }

    async fn cancel(&mut self, _downloader: Box<dyn Downloader>) {
        self.base.running = false;
    }

    async fn get_snapshot(&self) -> Option<Box<dyn std::any::Any>> {
        if let Some(ref status) = self.status {
            let current_speed = if let Some(ref monitor) = self.monitor {
                let stats = monitor.get_stats().await;
                stats.get("current_speed_bps").and_then(|v| v.as_f64()).unwrap_or(0.0)
            } else {
                0.0
            };

            let average_speed = if let Some(ref monitor) = self.monitor {
                let stats = monitor.get_stats().await;
                stats.get("average_speed_bps").and_then(|v| v.as_f64()).unwrap_or(0.0)
            } else {
                0.0
            };

            let snapshot = status.snapshot(current_speed, average_speed).await;
            Some(Box::new(snapshot))
        } else {
            None
        }
    }
}

impl HTTPDownloader {
    fn clone_downloader(&self) -> Self {
        HTTPDownloader {
            base: BaseDownloader {
                config: self.base.config.clone(),
                running: self.base.running,
                ..Default::default()
            },
            client: self.client.clone(),
            monitor: self.monitor.clone(),
            status: None,
        }
    }
}