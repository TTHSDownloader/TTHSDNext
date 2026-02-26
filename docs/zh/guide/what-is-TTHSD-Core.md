# 什么是 TTHSD 核心？

## 概述

**TTHSD 核心**（TT High Speed Downloader Core）是一个高性能、跨平台、多语言可调用的下载引擎内核，可为外部项目提供强大的下载能力支持，使开发者能够在自己的应用中轻松集成专业级的文件下载功能。

> [!TIP]
> 本文档介绍的是 **TTHSD Next**（Rust 版本），它是 [TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 的 Rust 重写版本。
> 
> TTHSD Next 的调用方式与 TTHSD Golang 相同，但性能更好。
> 
> 注：[TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 已停止开发，建议新项目使用 TTHSD Next。

## 核心概念

### 1. 高性能下载引擎
TTHSD 核心采用 **Rust 语言**编写，利用 Rust 的异步特性和零成本抽象实现了真正的多线程下载。它能够：
- 自动分割大文件为多个小块并行下载
- 智能调整线程数量以最大化带宽利用率
- **无 GC 暂停**：相比 Golang 版本，拥有更稳定的性能表现
- **更低的内存占用**：内存管理更加高效

### 2. 跨平台支持
TTHSD 核心编译为平台原生的动态链接库：
- **Windows**: ***tthsd.dll***
- **Linux**: ***tthsd.so***
- **macOS**: ***tthsd.dylib***
- **Android**: ***tthsd_android.so*** (通过 JNI 编译)
- **鸿蒙**: ***tthsd_harmony.so*** （通过 OpenHarmony SDK/NDK 编译）

这意味着您的应用程序可以在任何主流操作系统上运行，无需为不同平台重写下载逻辑。

### 3. 多语言调用接口
TTHSD 核心通过标准的 C ABI（应用程序二进制接口）暴露功能，支持：
- **Python**:  通过 我的接口封装 调用，性能无损
- **C/C++**:  通过 我的接口封装 调用
- **C#**:  通过 我的接口封装 调用
- **Java**:  通过 我的接口封装 调用（包括 Android 平台）
- **TypeScript/JavaScript**: 通过 我的接口封装 调用
- **其他任何支持调用 C 函数的语言**

### 4. 零性能损失的 Python 封装
TTHSD 提供了官方 Python 接口封装 ***TTHSD_interface.py***，其特点包括：
- **直接 C 调用**: 使用 ctypes 直接调用底层 C 函数，无中间层性能损失
- **类型安全**: 完整的类型注解和错误处理
- **异步回调**: 下载进度通过回调函数异步通知，不阻塞主线程
- **内存安全**: 自动管理 C 内存，避免内存泄漏

## TTHSD Next vs TTHSD Golang

### TTHSD Next (Rust 版本)
- **更高的下载速度**：优化的异步 I/O 模型
- **更低的内存占用**：精细的内存管理，无 GC 开销
- **更高的并发数**：可支持数十万并发连接
- **更安全的内存管理**：Rust 的所有权机制确保内存安全
- **新增 Android 支持**：提供完整的 JNI 接口
- **更稳定的性能**：无 GC 暂停，性能可预测

### TTHSD Golang (已停止开发)
- 调用方式与 TTHSD Next 相同
- 已停止维护和更新
- 不再接收新功能和 Bug 修复
- 建议新项目迁移到 TTHSD Next

## 为什么要使用 TTHSD 核心？

### 1. 专注于业务逻辑，而非下载实现
作为开发者，您可能面临以下挑战：
- 需要实现文件下载功能，但不想从头编写复杂的下载逻辑
- 担心自己实现的下载器性能不佳、不稳定
- 需要支持多平台，但不想为每个平台维护不同的代码
- 希望提供良好的用户体验（进度显示、暂停/恢复、错误处理等）

TTHSD 核心解决了所有这些痛点，让您能够：
```python
# 只需几行代码即可集成专业级下载功能
from TTHSD_interface import TTHSDownloader

downloader = TTHSDownloader()
downloader.start_download(
    urls=["https://example.com/file.zip"],
    save_paths=["./downloads/file.zip"],
    callback=progress_callback
)
```

### 2. 性能优势
- **原生性能**: Rust 语言编译的二进制代码，执行效率高
- **真正的并发**: 利用 Rust 的 async/await 和 tokio 运行时实现真正的并行下载
- **智能分块**: 根据文件大小和网络状况自动优化分块策略
- **零拷贝传输**: 内存缓冲区直接传输，减少数据复制开销
- **无 GC 暂停**: 相比 Golang 版本，性能更加稳定可预测

### 3. 稳定性和可靠性
- **经过测试**: 核心代码经过严格测试，处理各种网络异常
- **错误恢复**: 自动重试失败的分块，确保下载完成
- **资源管理**: 自动管理连接池和内存，避免资源泄漏
- **线程安全**: API 设计为线程安全，支持并发调用
- **内存安全**: Rust 的所有权机制确保内存安全，避免常见的内存错误

### 4. 开发效率
- **快速集成**: 只需引入一个动态库和一个 Python 文件
- **完善文档**: 详细的 API 文档和示例代码
- **活跃维护**: 持续更新和改进，修复问题及时
- **社区支持**: 开源项目，有问题可以查看源码或提交 issue

## 技术架构

|                   架构                   |
|------------------------------------------|
|     您的应用程序 (Python/C++/C#/Java)    |
|                   (↓)                   |
|         ( TTHSD Python 接口封装 )        |
|                    ↓                    |
|         C ABI 接口 (标准调用约定)        |
|                    ↓                    |
|   TTHSD 核心引擎 (Rust 编译的动态链接库) |
|                    ↓                    |
|  多线程下载管理器 / 网络层 / 文件 IO 层  |

### 核心组件
1. **下载任务调度器**: 管理多个下载任务，分配线程资源
2. **HTTP 客户端**: 处理 HTTP/HTTPS 协议，支持 Range 请求
3. **分块管理器**: 将大文件分割为小块并行下载
4. **进度监控器**: 实时跟踪下载进度和速度
5. **事件系统**: 通过回调函数通知下载状态变化
6. **错误处理器**: 处理网络异常、文件错误等

### 使用的 Rust 库
- **tokio**: 异步运行时，提供高效的异步 I/O
- **reqwest**: HTTP 客户端，支持异步请求
- **tokio-tungstenite**: WebSocket 客户端
- **serde / serde_json**: 序列化/反序列化
- **async-trait**: 异步 trait 支持
- **jni**: Android JNI 支持（可选）

## 适用场景

### 1. 桌面应用程序
- 软件更新器
- 资源下载器
- 媒体文件下载工具

### 2. 后端服务
- 批量文件处理
- 数据同步服务
- 内容分发网络

### 3. 游戏开发
- 游戏资源下载
- 补丁更新
- DLC 内容下载

### 4. 移动应用
- 通过 JNI 接口在 Android 应用中调用
- 后台下载任务

## 开始使用

### 基本要求
- 目标平台: Windows/Linux/macOS/Android
- Python 3.11+ (如果使用 Python 接口)
- 动态链接库文件

### 快速集成
1. 下载对应平台的动态链接库
2. 将库文件放入项目目录
3. (Python) 复制 ***TTHSD_interface.py*** 到项目
4. 按照文档调用 API

### 示例工作流
```python
# 1. 初始化
from TTHSD_interface import TTHSDownloader
downloader = TTHSDownloader()

# 2. 定义进度回调
def progress_callback(event, msg):
    if event['Type'] == 'update':
        print(f"进度: {msg['Downloaded']}/{msg['Total']}")

# 3. 开始下载
downloader.start_download(
    urls=["https://example.com/large-file.iso"],
    save_paths=["./downloads/file.iso"],
    thread_count=32,
    callback=progress_callback
)
```

## 总结

TTHSD 核心是一个专业级的下载解决方案，它提供了：

|  功能            |  描述                                |
|------------------|--------------------------------------|
|  ✅ **高性能**  |  真正的多线程下载，最大化带宽利用率  |
|  ✅ **跨平台**  |  一次集成，全平台运行（含 Android）  |
|  ✅ **多语言**  |  支持几乎所有主流编程语言            |
|  ✅ **易用性**  |  简单的 API，快速集成                |
|  ✅ **稳定性**  |  经过测试，处理各种异常情况          |
|  ✅ **开源**    |  透明可控，可根据需要修改            |

无论您是在开发桌面应用、后端服务还是移动应用，TTHSD 核心都能为您提供可靠、高效的下载功能，让您专注于核心业务逻辑的开发。

---

**下一步**:
- [快速开始](/zh/guide/getting-started) - 立即开始使用 TTHSD
- [API 详解](/zh/api/API-overview) - 了解所有可用功能
- [Python 接口使用](/zh/interface/py/how-to-use) - Python 开发指南
- [TypeScript 接口使用](/zh/interface/ts/how-to-use) - TypeScript 开发指南
