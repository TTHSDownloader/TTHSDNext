# 快速开始

> [!WARNING]
> 注意：本文档及其以下所有文档主要针对 **TTHSD Next**（Rust 版本）
> 
> 如果您使用的是 **TTHSD Golang** 版本，请注意：
> - [TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 已停止开发
> - 建议迁移到 TTHSD Next 以获得更好的性能和功能
> - 调用方式与 TTHSD Next 相同，可以直接替换动态库
> - [TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 的官方发布最新版（ 0.5.0 版本 ）有 **Bug** ，不建议使用。

## 简介

TT High Speed Downloader (TTHSD) 是一个高性能的多线程下载器内核，支持 HTTP/HTTPS 等协议。TTHSD Next 采用 **Rust 语言**编写，核心逻辑编译为动态链接库 (DLL/SO/DYLIB)，可供 Python、C++、C#、TypeScript 等多种语言调用。

相比 [TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 版本，TTHSD Next 具有更高的性能、更低的内存占用和更稳定的运行表现。

本文档将指导您如何在 Python 项目中快速集成和使用 TTHSD Next。

## 安装指南

### 1. 系统要求

- **操作系统**: Windows (x64), Linux (Ubuntu 22.04+), macOS, Android
- **Python 版本**: Python 3.11 及以上
- **依赖项**: 不需要额外安装 Python 依赖库，通过 ***ctypes*** 直接调用内核。

### 2. 获取内核文件

您需要下载对应平台的动态链接库文件：

- **Windows**: ***TTHSD.dll***
- **Linux**: ***libTTHSD.so***
- **macOS**: ***libTTHSD.dylib***
- **Android**: ***libTTHSD.so*** (通过 JNI 接口)

> [!TIP]
> 您可以在 [TTHSD Next GitHub Releases](https://github.com/sxxyrry/TTHSDNext/releases/latest) 页面下载最新版本的内核文件。

### 3. 获取 Python 接口封装

为了简化调用，官方提供了封装好的 Python 接口文件 ***TTHSD_interface.py***。

1. 下载 ***TTHSD_interface.py***。
2. 将 **内核文件** 和 ***TTHSD_interface.py*** 放置在您的 Python 项目根目录下。

## 基本用法

以下示例展示了如何初始化下载器并下载一个文件。

### 初始化与回调

首先，导入模块并定义一个回调函数来接收下载进度和状态。

```python
from TTHSD_interface import TTHSDownloader
import json

# 1. 实例化下载器
# 确保 dll/so 文件在当前目录或系统路径中
downloader = TTHSDownloader()

# 2. 定义回调函数
# event_dict: 包含事件类型(Type)、任务ID(ID)等元数据
# msg_dict: 包含具体的进度数据(Total, Downloaded)或错误信息
def progress_callback(event_dict, msg_dict):
    event_type = event_dict.get("Type")
    
    if event_type == "update":
        # 进度更新
        total = msg_dict.get("Total", 0)
        downloaded = msg_dict.get("Downloaded", 0)
        if total > 0:
            percent = (downloaded / total) * 100
            print(f"\r进度: {percent:.2f}% ({downloaded}/{total}) 字节", end="")
            
    elif event_type == "endOne":
        print(f"\n文件下载完成: {msg_dict.get('ShowName')}")
        
    elif event_type == "msg":
        print(f"\n消息: {msg_dict['Text']}")
    
    elif event_type == "err":
        print(f"\n错误: {msg_dict['Error']}")

# 3. 启动下载
# start_download 会返回一个下载器 ID
task_id = downloader.start_download(
    urls=["https://example.com/file.zip"],
    save_paths=["./downloads/file.zip"],
    thread_count=16,      # 线程数 (默认根据 CPU 核心数自动调整)
    chunk_size_mb=10,     # 分块大小 (MB)
    callback=progress_callback,
    is_multiple=False     # 是否并行下载 (False 为顺序下载)
)

print(f"下载任务已启动，ID: {task_id}")

# 注意：主程序不能立即退出，否则后台下载线程会被终止。
# 在实际 GUI 程序中不需要此步，但在脚本中需要阻塞主线程。
input("按 Enter 退出...")
```

## TTHSD Next 的优势

相比 TTHSD Golang 版本，TTHSD Next 提供以下改进：

### 性能提升
- **更高的下载速度**：优化的异步 I/O 模型
- **更低的内存占用**：精细的内存管理，无 GC 开销
- **更高的并发数**：可支持数十万并发连接
- **更稳定的性能**：无 GC 暂停，性能可预测

### 新增功能
- **Android 支持**：提供完整的 JNI 接口，可在 Android 应用中调用
- **更安全的内存管理**：Rust 的所有权机制确保内存安全

### 兼容性
- **调用方式相同**：API 接口与 TTHSD Golang 完全兼容
- **直接替换**：只需替换动态库文件即可迁移

## 下一步

- 了解 [ API 详解](/zh/api/API-overview) 以掌握更多高级功能。
- 查看 [Python 接口使用文档](/zh/interface/py/how-to-use) 了解完整的 Python API。
- 查看 [TypeScript 接口使用文档](/zh/interface/ts/how-to-use) 了解如何在 Node.js 中使用。
- 访问 [TTHSD Next GitHub](https://github.com/sxxyrry/TTHSDNext) 获取源代码和更多信息。