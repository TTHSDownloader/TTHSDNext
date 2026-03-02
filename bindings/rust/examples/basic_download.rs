//! Rust 绑定 Crate 使用示例
//!
//! 运行方式（先将 libtthsd.so 放到工作目录）:
//! ```bash
//! cargo run --example basic_download
//! ```

use tthsd::{TTHSDownloader, DownloadOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 加载动态库（None = 在当前目录搜索 libtthsd.so/.dll/.dylib）
    let dl = TTHSDownloader::load(None)?;

    // 2. 启动下载，获取事件接收 channel
    let (id, mut rx) = dl.start_download(
        vec!["https://example.com/bigfile.zip".to_string()],
        vec!["/tmp/bigfile.zip".to_string()],
        DownloadOptions {
            thread_count: Some(32),
            chunk_size_mb: Some(10),
            ..Default::default()
        },
    )?;

    println!("✅ 下载器已启动，ID = {}", id);

    // 3. 异步接收事件
    while let Some(msg) = rx.recv().await {
        match msg.event.event_type.as_str() {
            "start" => {
                println!("🚀 下载会话开始");
            }
            "startOne" => {
                let index = msg.data.get("Index").and_then(|v| v.as_i64()).unwrap_or(0);
                let total = msg.data.get("Total").and_then(|v| v.as_i64()).unwrap_or(0);
                let url   = msg.data.get("URL").and_then(|v| v.as_str()).unwrap_or("");
                println!("▶ 开始下载 [{}/{}]: {}", index, total, url);
            }
            "update" => {
                let downloaded = msg.data.get("Downloaded").and_then(|v| v.as_i64()).unwrap_or(0);
                let total      = msg.data.get("Total").and_then(|v| v.as_i64()).unwrap_or(1);
                let pct = downloaded as f64 / total as f64 * 100.0;
                print!("\r进度: {}/{} ({:.2}%)", downloaded, total, pct);
            }
            "endOne" => {
                println!("\n✅ 单文件完成: {}", msg.event.show_name);
            }
            "end" => {
                println!("\n🏁 全部下载完成");
                break;
            }
            "err" => {
                let err = msg.data.get("Error").and_then(|v| v.as_str()).unwrap_or("未知错误");
                eprintln!("\n❌ 错误: {}", err);
                break;
            }
            _ => {}
        }
    }

    // 4. 停止下载器（end 事件后 DLL 会自动清理，这里显式调用也可以）
    dl.stop_download(id);
    Ok(())
}
