use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use super::websocket_client::WebSocketClient;
use super::socket_client::SocketClient;
use super::send_message::send_message;
use super::performance_monitor::get_global_monitor;

pub const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub url: String,
    pub save_path: String,
    pub show_name: String,
    pub id: String,
}

pub type ProgressCallback = extern "C" fn(*const std::ffi::c_char, *const std::ffi::c_char);

#[derive(Debug, Clone)]
pub struct DownloadConfig {
    pub tasks: Vec<DownloadTask>,
    pub thread_count: usize,
    pub chunk_size_mb: usize,
    pub callback_func: Option<ProgressCallback>,
    pub use_callback_url: bool,
    pub callback_url: Option<String>,
    pub use_socket: Option<bool>,
    pub show_name: String,
    pub user_agent: String,
}

#[derive(Debug, Clone)]
pub struct DownloadChunk {
    pub start_offset: i64,
    pub end_offset: i64,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventType {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "startOne")]
    StartOne,
    #[serde(rename = "update")]
    Update,
    #[serde(rename = "end")]
    End,
    #[serde(rename = "endOne")]
    EndOne,
    #[serde(rename = "msg")]
    Msg,
    #[serde(rename = "err")]
    Err,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    #[serde(rename = "Type")]
    pub event_type: EventType,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "ShowName")]
    pub show_name: String,
    #[serde(rename = "ID")]
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct ProgressEvent {
    pub total: i64,
    pub downloaded: i64,
}

pub struct HSDownloader {
    pub config: Arc<RwLock<DownloadConfig>>,
    pub ws_client: Option<Arc<tokio::sync::Mutex<WebSocketClient>>>,
    pub socket_client: Option<Arc<tokio::sync::Mutex<SocketClient>>>,
    pub cancel_token: Arc<tokio::sync::Mutex<Option<tokio_util::sync::CancellationToken>>>,
    pub current_task_index: Arc<tokio::sync::Mutex<usize>>,
}

impl HSDownloader {
    pub fn new(config: DownloadConfig) -> Self {
        let config = Arc::new(RwLock::new(config));

        let (ws_client, socket_client) = {
            let cfg = config.blocking_read();
            let mut ws_client = None;
            let mut socket_client = None;

            if cfg.use_callback_url {
                if let Some(ref callback_url) = cfg.callback_url {
                    if let Some(use_socket) = cfg.use_socket {
                        if use_socket {
                            socket_client = Some(Arc::new(tokio::sync::Mutex::new(SocketClient::new(callback_url.clone()))));
                        } else {
                            ws_client = Some(Arc::new(tokio::sync::Mutex::new(WebSocketClient::new(callback_url.clone()))));
                        }
                    }
                }
            }
            (ws_client, socket_client)
        };

        HSDownloader {
            config,
            ws_client,
            socket_client,
            cancel_token: Arc::new(tokio::sync::Mutex::new(None)),
            current_task_index: Arc::new(tokio::sync::Mutex::new(0)),
        }
    }

    pub fn get_downloader(tasks: Vec<DownloadTask>, thread_count: usize, chunk_size_mb: usize) -> Self {
        let num_cpus = num_cpus::get();
        let thread_count = if thread_count == 0 { num_cpus * 2 } else { thread_count };
        let chunk_size_mb = if chunk_size_mb == 0 { 10 } else { chunk_size_mb };

        let config = DownloadConfig {
            tasks,
            thread_count,
            chunk_size_mb,
            callback_func: None,
            use_callback_url: false,
            callback_url: None,
            use_socket: None,
            show_name: String::new(),
            user_agent: UA.to_string(),
        };

        Self::new(config)
    }

    pub async fn start_download(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut cancel_guard = self.cancel_token.lock().await;
        if cancel_guard.is_some() {
            drop(cancel_guard);
            return Err("downloader already running".into());
        }

        let token = tokio_util::sync::CancellationToken::new();
        *cancel_guard = Some(token.clone());
        drop(cancel_guard);

        let event = Event {
            event_type: EventType::Start,
            name: "开始下载".to_string(),
            show_name: "全局".to_string(),
            id: String::new(),
        };

        send_message(event, HashMap::new(), &self.config, &self.ws_client, &self.socket_client).await?;

        let tasks = {
            let config = self.config.read().await;
            config.tasks.clone()
        };

        let mut join_set = tokio::task::JoinSet::new();

        for (index, task) in tasks.into_iter().enumerate() {
            let token_clone = token.clone();
            let config = self.config.clone();
            let ws_client = self.ws_client.clone();
            let socket_client = self.socket_client.clone();

            join_set.spawn(async move {
                Self::download_task(
                    task,
                    index,
                    token_clone,
                    config,
                    ws_client,
                    socket_client,
                ).await
            });
        }

        while let Some(result) = join_set.join_next().await {
            if let Err(e) = result {
                eprintln!("Task failed: {:?}", e);
            }
        }

        let end_event = Event {
            event_type: EventType::End,
            name: "结束所有下载".to_string(),
            show_name: "全局".to_string(),
            id: String::new(),
        };

        send_message(end_event, HashMap::new(), &self.config, &self.ws_client, &self.socket_client).await?;

        if let Some(monitor) = get_global_monitor().await {
            monitor.print_stats().await;
        }

        let mut cancel_guard = self.cancel_token.lock().await;
        *cancel_guard = None;
        drop(cancel_guard);

        Ok(())
    }

    pub async fn start_multiple_downloads(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut cancel_guard = self.cancel_token.lock().await;
        if cancel_guard.is_some() {
            drop(cancel_guard);
            return Err("downloader already running".into());
        }

        let token = tokio_util::sync::CancellationToken::new();
        *cancel_guard = Some(token.clone());
        drop(cancel_guard);

        let event = Event {
            event_type: EventType::Start,
            name: "开始批量下载".to_string(),
            show_name: "全局".to_string(),
            id: String::new(),
        };

        send_message(event, HashMap::new(), &self.config, &self.ws_client, &self.socket_client).await?;

        let tasks = {
            let config = self.config.read().await;
            config.tasks.clone()
        };

        let mut join_set = tokio::task::JoinSet::new();

        for (index, task) in tasks.into_iter().enumerate() {
            let token_clone = token.clone();
            let config = self.config.clone();
            let ws_client = self.ws_client.clone();
            let socket_client = self.socket_client.clone();

            join_set.spawn(async move {
                Self::download_task(
                    task,
                    index,
                    token_clone,
                    config,
                    ws_client,
                    socket_client,
                ).await
            });
        }

        while let Some(result) = join_set.join_next().await {
            if let Err(e) = result {
                eprintln!("Task failed: {:?}", e);
            }
        }

        let end_event = Event {
            event_type: EventType::End,
            name: "结束批量下载".to_string(),
            show_name: "全局".to_string(),
            id: String::new(),
        };

        send_message(end_event, HashMap::new(), &self.config, &self.ws_client, &self.socket_client).await?;

        let mut cancel_guard = self.cancel_token.lock().await;
        *cancel_guard = None;
        drop(cancel_guard);

        Ok(())
    }

    async fn download_task(
        task: DownloadTask,
        index: usize,
        token: tokio_util::sync::CancellationToken,
        config: Arc<RwLock<DownloadConfig>>,
        ws_client: Option<Arc<Mutex<WebSocketClient>>>,
        socket_client: Option<Arc<Mutex<SocketClient>>>,
    ) {
        let total = {
            let cfg = config.read().await;
            cfg.tasks.len()
        };

        let start_event = Event {
            event_type: EventType::StartOne,
            name: "开始一个下载".to_string(),
            show_name: task.show_name.clone(),
            id: task.id.clone(),
        };

        let mut data = HashMap::new();
        data.insert("URL".to_string(), serde_json::Value::String(task.url.clone()));
        data.insert("SavePath".to_string(), serde_json::Value::String(task.save_path.clone()));
        data.insert("ShowName".to_string(), serde_json::Value::String(task.show_name.clone()));
        data.insert("Index".to_string(), serde_json::Value::Number(serde_json::Number::from(index + 1)));
        data.insert("Total".to_string(), serde_json::Value::Number(serde_json::Number::from(total)));

        if let Err(e) = send_message(start_event, data, &config, &ws_client, &socket_client).await {
            eprintln!("Failed to send start event: {:?}", e);
        }

        // 通过工厂函数获取下载器实例（支持多种下载器类型扩展）
        let err: Option<Box<dyn std::error::Error + Send + Sync>> = {
            let mut downloader = super::get_downloader::get_downloader(config.clone()).await;
            match downloader.download(&task).await {
                Ok(()) => None,
                Err(e) => {
                    eprintln!("下载失败 [{}]: {:?}", task.show_name, e);
                    Some(e)
                }
            }
        };

        let mut end_data = HashMap::new();
        end_data.insert("URL".to_string(), serde_json::Value::String(task.url));
        end_data.insert("SavePath".to_string(), serde_json::Value::String(task.save_path));
        end_data.insert("ShowName".to_string(), serde_json::Value::String(task.show_name.clone()));
        end_data.insert("Index".to_string(), serde_json::Value::Number(serde_json::Number::from(index + 1)));
        end_data.insert("Total".to_string(), serde_json::Value::Number(serde_json::Number::from(total)));

        if let Some(e) = err {
            if !token.is_cancelled() {
                let error_event = Event {
                    event_type: EventType::Err,
                    name: "错误".to_string(),
                    show_name: task.show_name.clone(),
                    id: task.id.clone(),
                };
                let mut error_data = HashMap::new();
                error_data.insert("Error".to_string(), serde_json::Value::String(format!("下载文件失败: {:?}", e)));

                let _ = send_message(error_event, error_data, &config, &ws_client, &socket_client).await;
            }
        }

        let end_event = Event {
            event_type: EventType::EndOne,
            name: "结束一个下载".to_string(),
            show_name: task.show_name,
            id: task.id,
        };

        let _ = send_message(end_event, end_data, &config, &ws_client, &socket_client).await;
    }

    pub async fn pause_download(&self) {
        let mut cancel_guard = self.cancel_token.lock().await;
        if let Some(token) = cancel_guard.take() {
            token.cancel();
        }
        drop(cancel_guard);

        let event = Event {
            event_type: EventType::Msg,
            name: "暂停".to_string(),
            show_name: "全局".to_string(),
            id: String::new(),
        };

        let mut data = HashMap::new();
        data.insert("Text".to_string(), serde_json::Value::String("下载已暂停".to_string()));

        let _ = send_message(event, data, &self.config, &self.ws_client, &self.socket_client).await;
    }

    pub async fn resume_download(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.start_download().await
    }

    pub async fn stop_download(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.pause_download().await;

        // 关闭网络连接
        if let Some(ref ws_client) = self.ws_client {
            let client = ws_client.lock().await;
            client.close();
        }

        if let Some(ref socket_client) = self.socket_client {
            let client = socket_client.lock().await;
            client.close();
        }

        let event = Event {
            event_type: EventType::Msg,
            name: "停止".to_string(),
            show_name: "全局".to_string(),
            id: String::new(),
        };

        let mut data = HashMap::new();
        data.insert("Text".to_string(), serde_json::Value::String("下载已停止".to_string()));

        send_message(event, data, &self.config, &self.ws_client, &self.socket_client).await?;

        Ok(())
    }

    pub async fn get_snapshot(&self, _task_id: &str) -> Option<HashMap<String, serde_json::Value>> {
        if let Some(monitor) = get_global_monitor().await {
            return Some(monitor.get_stats().await);
        }
        None
    }
}