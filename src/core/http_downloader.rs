use std::sync::Arc;
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

impl HTTPDownloader {
    pub async fn new(config: Arc<RwLock<DownloadConfig>>) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"));

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

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

    async fn download_chunk(
        &self,
        task: &DownloadTask,
        chunk: &DownloadChunk,
        downloaded_size: Arc<RwLock<i64>>,
        _total_size: i64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"));
        headers.insert(RANGE, HeaderValue::from_str(&format!("bytes={}-{}", chunk.start_offset, chunk.end_offset))?);
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

        writer.seek(std::io::SeekFrom::Start(chunk.start_offset as u64)).await?;

        const BATCH_UPDATE_THRESHOLD: i64 = 512 * 1024;
        let mut local_downloaded = 0i64;

        let mut stream = response.bytes_stream();

        while let Some(bytes_result) = stream.next().await {
            let bytes = bytes_result?;

            {
                let mut lr = last_read.write().await;
                *lr = Instant::now();
            }

            writer.write_all(&bytes).await?;

            local_downloaded += bytes.len() as i64;

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

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&task.save_path).await?;

        file.set_len(file_size as u64).await?;

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

        let mut join_set = tokio::task::JoinSet::new();

        for chunk in chunks {
            let task_clone = task.clone();
            let downloaded_size_clone = downloaded_size.clone();
            let self_clone = self.clone_downloader();

            join_set.spawn(async move {
                self_clone.download_chunk(&task_clone, &chunk, downloaded_size_clone, file_size).await
            });
        }

        while let Some(result) = join_set.join_next().await {
            if let Err(e) = result {
                self.send_error_message(format!("worker error: {:?}", e)).await;
                if let Some(ref status) = self.status {
                    status.set_error(format!("worker error: {:?}", e)).await;
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