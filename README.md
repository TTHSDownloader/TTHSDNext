# TTHSD Next (TT High Speed Downloader)

<div align="center">
  <img src="https://img.shields.io/badge/Rust-1.75+-orange.svg" alt="Rust Version">
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS%20%7C%20Android%20%7C%20HarmonyOS-blue.svg" alt="Platform">
  <img src="https://img.shields.io/badge/License-GPL--3.0-green.svg" alt="License">
</div>

**项目团队**：[查看文档](https://docss.sxxyrry.qzz.io/TTHSD/zh/acknowledgments/acknowledgments.html#%E9%A1%B9%E7%9B%AE%E5%9B%A2%E9%98%9F)

## 概述

**TTHSD 核心**（TT High Speed Downloader Core）是一个高性能、跨平台、多语言可调用的下载引擎内核，可为外部项目提供强大的下载能力支持，使开发者能够在自己的应用中轻松集成专业级的文件下载功能。

> [!TIP]
> 本项目是 **TTHSD Next**（Rust 版本），它是 [TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 的 Rust 重写版本。
> 
> TTHSD Next 的调用方式与 TTHSD Golang 相同，但性能更好。
> 
> 注：[TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 已停止开发，建议新项目使用 TTHSD Next。

### 和 Golang 的性能对比：

- 本版本的性能更高
- 本版本增加了 Android 的支持
- 本版本增加了 鸿蒙 的支持
- 本版本增加了 对绝大多数 Linux 发行版 的支持

## ✨ 核心特性

- ⚡️ **极致性能**: 纯 Rust 异步并发架构，基于 `tokio` 运行时和 `reqwest`，内存占用极低（通常 <20MB），并榨干网络和磁盘 IO 性能。
- 🧩 **高度解耦与扩展性**: 实现了下载器工厂模式（Downloader Factory），内部接口高度抽象，便于在未来无缝扩展 P2P、FTP、BT 等不同协议的下载支持。
- 🌍 **全量跨平台支持**:
  - **桌面平台**: Windows (x86_64, ARM64), Linux (x86_64, ARM64), macOS (Intel, Apple Silicon)
  - **移动平台**: Android (ARM64-v8a, ARMv7, x86_64)
  - **物联网与生态**: 华为 HarmonyOS / OpenHarmony (ARM64, x86_64)
- 🔌 **通用语言接口**: 原生提供标准的 C ABI（供 C/C++, Python, C# 甚至 Electron/Tauri 调用）和 JNI 接口（供 Android/Java/Kotlin 直接加载使用）。
- 🤖 **自动化 CI/CD**: 配置了完善的 GitHub Actions 工作流，每次提交和发布均自动完成 11 个跨平台标的的交叉编译和打包测试。

## 📦 发行版下载与目录结构

在 GitHub 的 `Releases` 页面中，你可以下载到开箱即用的极限压缩发行包 `TTHSD_Release.7z`，其结构如下：

```text
📁 TTHSD_Release/
 ├── 📁 desktop/
 │    ├── tthsd.dll           # Windows x86_64 动态库
 │    ├── tthsd_arm64.dll     # Windows ARM64 动态库
 │    ├── tthsd.so            # Linux x86_64 动态库
 │    ├── tthsd_arm64.so      # Linux ARM64 动态库
 │    ├── tthsd.dylib         # macOS Intel 动态库
 │    └── tthsd_arm64.dylib   # macOS Silicon 动态库
 ├── 📁 android/
 │    ├── tthsd_android_arm64.so # Android ARM64 库
 │    ├── tthsd_android_armv7.so # Android 32位 库
 │    └── tthsd_android_x86_64.so# Android 模拟器库
 ├── 📁 harmony/
 │    └── tthsd_harmony_arm64.so # HarmonyOS ARM64 库
 └── 📁 scripts/
      ├── TTHSD_interface.py     # Python 接口封装和调用示例
      └── test_comprehensive.py  # 本地全量性能与压力测试套件
```

## 🚀 快速上手 (Python 示例)

你可以通过内置的 `ctype` 直接调用 TTHSD。我们提供了 `TTHSD_interface.py` 方便你直接集成进 Python 项目中。

```python
from scripts.TTHSD_interface import TTHSDownloader, EventLogger

# 1. 实例化下载器，传入对应平台的动态库路径
downloader = TTHSDownloader('./desktop/tthsd.dll') # Windows为例

# 2. 定义回调接口接收异步事件信息
logger_callback = EventLogger()

# 3. 发起下载任务
task_id = downloader.start_download(
    urls=["https://example.com/large_file.zip"],
    save_paths=["./downloads/large_file.zip"],
    thread_count=16,          # 启用 16 个并发分片线程
    chunk_size_mb=4,          # 每个分片 4MB 大小
    callback=logger_callback
)

print(f"📦 开始下载，任务内部ID: {task_id}")

# ...等待下载完成...
downloader.close()
```

## 🛠️ 本地编译指南

此项目使用 `cargo` 作为主要的构建系统。如果你想亲自从源码开始编译：

### 环境准备

1. **安装 Rust**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
2. **下载源码**:
   ```bash
   git clone https://github.com/YourUsername/TTHSD.git
   cd TTHSD/TTHSD
   ```

### 编译为本地平台库

进入内部的 `TTHSD` 模块后运行:

```bash
cargo build --release
```
编译产物位于 `target/release/` 下（Linux为 `.so`，Windows为 `.dll`，macOS为 `.dylib`）。

### 交叉编译至移动端或其它系统

1. **Android**: 使用 `cargo-ndk`
   ```bash
   cargo install cargo-ndk
   rustup target add aarch64-linux-android
   cargo ndk --target arm64-v8a --platform 21 build --release --features android
   ```
2. **HarmonyOS**: 需要配置对应的 OHOS SDK NDK。详细的链接器配置要求请参考该项目 Actions 工作流配置 `.github/workflows/build_and_test.yml` 中的内容。

## 🧪 自动化测试套件

为了验证并发极限和各类边界逻辑，通过运行 Python 测试套件模拟了从微文件到上百 MB 的大文件断点和压力验证。

```bash
# 后台启动本地 HTTP 测速桩
python3 scripts/test_server.py &

# 执行全量自动化综合测试
python3 scripts/test_comprehensive.py
```
*套件包含并支持:*
- 并发请求吞吐量验证
- 下载一致性检验 (MD5/SHA)
- `Content-Range` HTTP协议标准容错行为
- 运行时内存泄漏监测

## 📄 开源许可证

本项目基于 [GNU General Public License v3.0 (GPL-3.0)](LICENSE) 协议发布。这保证了核心软件始终维持开源与自由复制的属性，修改请务必同等开源并保留原作者声明。
