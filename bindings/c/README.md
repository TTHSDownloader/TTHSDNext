# TTHSD C / C++ 封装

> 为 C、C++ 提供的 TTHSD 高速下载器接口封装。

---

## 📁 文件结构

| 文件 | 说明 |
|------|------|
| `tthsd.h` | 标准 C 头文件——声明所有 C ABI 导出函数及回调类型 |
| `TTHSDownloader.hpp` | C++ header-only 封装类——RAII 持有库句柄，`std::function` 回调 |
| `example/main.cpp` | C++ 示例程序 |
| `example/CMakeLists.txt` | CMake 构建配置 |

---

## C 接口 (`tthsd.h`)

### 回调类型

```c
typedef void (*TTHSD_Callback)(const char* event_json, const char* data_json);
```

回调参数均为 JSON 字符串：
- **`event_json`**: 事件元数据，包含 `Type`、`Name`、`ShowName`、`ID` 字段
- **`data_json`**: 附带数据，根据事件类型包含 `Downloaded`/`Total`（进度）或 `Error`（错误）等

### 导出函数

| 函数 | 说明 | 返回值 |
|------|------|--------|
| `start_download(...)` | 创建并**立即启动**下载器 | 下载器 ID（正整数），失败返回 -1 |
| `get_downloader(...)` | 创建下载器实例（**不启动**） | 同上 |
| `start_download_id(id)` | 按 ID 顺序启动下载 | 0=成功，-1=失败 |
| `start_multiple_downloads_id(id)` | 按 ID 并行启动下载 | 0=成功，-1=失败 |
| `pause_download(id)` | 暂停下载 | 0=成功，-1=失败 |
| `resume_download(id)` | 恢复下载（核心 ≥0.5.1） | 0=成功，-1=失败 |
| `stop_download(id)` | 停止并销毁下载器 | 0=成功，-1=失败 |

### `start_download` 参数

```c
int start_download(
    const char*     tasks_data,          // 任务列表 JSON 字符串
    int             task_count,          // 任务数量
    int             thread_count,        // 下载线程数
    int             chunk_size_mb,       // 分块大小（MB）
    TTHSD_Callback  callback,            // 回调函数指针（可为 NULL）
    bool            use_callback_url,    // 是否启用远程回调
    const char*     user_agent,          // 自定义 UA（可为 NULL）
    const char*     remote_callback_url, // 远程回调 URL（可为 NULL）
    const bool*     use_socket,          // 是否使用 Socket（可为 NULL）
    const bool*     is_multiple          // 是否并行多任务（可为 NULL）
);
```

### 任务 JSON 格式

```json
[
  {
    "url": "https://example.com/file.zip",
    "save_path": "/tmp/file.zip",
    "show_name": "file.zip",
    "id": "uuid-string"
  }
]
```

---

## C++ 封装 (`TTHSDownloader.hpp`)

Header-only 封装，依赖 [nlohmann/json](https://github.com/nlohmann/json)，要求 **C++17** 及以上。

### 特性

- **RAII**：构造时 `load()` 加载库，析构时自动卸载
- **跨平台**：自动选择 `LoadLibrary` (Windows) 或 `dlopen` (Linux/macOS)
- **回调**：使用 `std::function<void(const json&, const json&)>` 接收事件

### 快速开始

```cpp
#include "TTHSDownloader.hpp"

int main() {
    TTHSDownloader dl;
    dl.load();  // 自动搜索 TTHSD.dll / TTHSD.so / TTHSD.dylib

    int id = dl.startDownload(
        {"https://example.com/a.zip"},
        {"/tmp/a.zip"},
        {.threadCount = 32},
        [](const json& event, const json& data) {
            if (event["Type"] == "update")
                std::cout << "进度: " << data["Downloaded"] << "/" << data["Total"] << "\n";
        }
    );

    std::cout << "下载 ID: " << id << std::endl;
    // 主线程需保持运行，否则后台线程会被终止
    std::cin.get();
    dl.stopDownload(id);
}
```

### API 参考

| 方法 | 说明 |
|------|------|
| `void load(path)` | 加载动态库（空路径自动搜索） |
| `int startDownload(urls, paths, params, callback)` | 创建并启动下载 |
| `int getDownloader(urls, paths, params, callback)` | 创建下载器（不启动） |
| `bool startDownloadById(id)` | 顺序启动 |
| `bool startMultipleDownloadsById(id)` | 并行启动 |
| `bool pauseDownload(id)` | 暂停 |
| `bool resumeDownload(id)` | 恢复 |
| `bool stopDownload(id)` | 停止并销毁 |

### `DownloadParams` 结构体

```cpp
struct DownloadParams {
    int threadCount     = 64;
    int chunkSizeMB     = 10;
    bool useCallbackUrl = false;
    std::string userAgent;
    std::string remoteCallbackUrl;
    bool* useSocket   = nullptr;
    bool* isMultiple  = nullptr;
};
```

---

## 构建示例

```bash
cd example
mkdir build && cd build
cmake ..
cmake --build .
./tthsd_example
```

---

## 事件类型

| Type | 说明 | data 字段 |
|------|------|-----------|
| `start` | 下载会话开始 | — |
| `startOne` | 单个任务开始 | `URL`, `SavePath`, `ShowName`, `Index`, `Total` |
| `update` | 进度更新 | `Downloaded`, `Total` |
| `endOne` | 单个任务完成 | `URL`, `SavePath`, `ShowName`, `Index`, `Total` |
| `end` | 全部任务完成 | — |
| `msg` | 消息通知 | `Text` |
| `err` | 错误 | `Error` |
