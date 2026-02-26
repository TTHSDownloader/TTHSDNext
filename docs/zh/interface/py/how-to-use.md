# TT 高速下载器 Python 接口封装使用文档

> [!TIP]
> 本文档介绍的是 **TTHSD Next**（Rust 版本）的 Python 接口。
> 
> API 接口与 [TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 版本完全兼容，只需替换动态库即可迁移。
> 
> TTHSD Golang 已停止开发，建议使用 TTHSD Next 以获得更好的性能。

本文档介绍 ***TTHSD_interface.py*** 模块，该模块为 **TT高速下载器**（***TTHSD.dll***/***.so***/***.dylib***）提供 Python 封装，支持多任务下载的创建、控制与进度回调。

---

## 0. 安装说明

### 0.1 安装依赖

1. Python 3.11 及以上版本
2. TT高速下载器动态库文件：
   - Windows 平台：***TTHSD.dll***
   - macOS 平台：***TTHSD.dylib***
   - Linux 平台：***TTHSD.so***

### 0.2 安装步骤

1. 将动态库文件放置在项目目录中
2. 将 ***TTHSD_interface.py*** 复制到项目目录

---

## 1. 模块概述

- **核心类**：***TTHSDownloader***
- **依赖**：***ctypes***、***json***、***threading***、***queue***、***weakref*** 等标准库
- **功能**：
  - 创建下载器实例（立即启动或只创建）
  - 控制下载（开始、暂停、恢复、停止）
  - 通过回调函数接收下载进度、事件通知（***update***、***endOne***、***end***、***msg***、***err*** 等）
  - 内置线程安全、异步日志记录

---

## 2. ***TTHSDownloader*** 类

### 2.1 初始化
```python
downloader = TTHSDownloader(dll_path: str | pathlib.Path | None = None)
```
- **参数** ***dll_path***：动态库路径。若为 ***None***，根据操作系统自动猜测默认文件名（Windows: ***TTHSD.dll***，macOS: ***TTHSD.dylib***，Linux: ***TTHSD.so***），路径为当前工作目录。

### 2.2 创建下载器
#### ***start_download(...) -> int***
立即创建并启动下载器。
```python
def start_download(
    urls: list[str],
    save_paths: list[str],
    thread_count: int = 64,
    chunk_size_mb: int = 10,
    callback: Callable[[dict, dict], None] | None = None,
    use_callback_url: bool = False,
    user_agent: str | None = None,
    remote_callback_url: str | None = None,
    use_socket: bool | None = None,
    is_multiple: bool | None = None
) -> int
```
- **返回**：下载器实例 ID（正整数），失败时返回 -1。

#### ***get_downloader(...) -> int***
仅创建下载器（不启动），返回实例 ID。参数同 ***start_download***（不含 ***is_multiple***）。

### 2.3 控制下载
所有控制方法均返回 ***bool***，***True*** 表示操作成功（DLL 返回 0），***False*** 表示失败（如 ID 不存在）。
- ***start_download_by_id(downloader_id: int) -> bool***：开始顺序下载
- ***start_multiple_downloads_by_id(downloader_id: int) -> bool***：开始并行下载（实验性）
- ***pause_download(downloader_id: int) -> bool***：暂停下载
- ***resume_download(downloader_id: int) -> bool***：恢复下载
- ***stop_download(downloader_id: int) -> bool***：停止下载

### 2.4 资源管理
Python 接口会自动管理回调函数引用，无需手动清理资源。当下载器实例被销毁时，相关引用会被自动释放。

#### 上下文管理器支持
```python
with TTHSDownloader() as d:
    d.start_download(...)
    # 退出 with 块时自动清理资源
```

注意：当前实现不支持显式的 ***close()*** 方法，资源由 Python 垃圾回收机制自动管理。

---

## 3. 回调函数

### 3.1 定义格式
用户提供的回调函数需接受两个字典参数：
```python
def my_callback(event: dict, msg: dict) -> None:
    ...
```
- **event**：包含事件类型、名称等元数据
- **msg**：携带具体数据（进度、结果、错误等）

### 3.2 事件类型与数据格式 

本部分与 [Event 文档](/zh/event/event-overview) 相同，在此不过多赘述

### 3.3 线程安全要求
- 回调函数在 **DLL 内部的工作线程** 中执行。
- **禁止在回调中执行耗时操作**（如磁盘写入、网络请求），也**禁止直接更新 GUI**（应通过队列发送到主线程）。
- 如果多个下载器共享同一个回调，需自行加锁保护共享数据。

---

## 4. 使用示例

### 4.1 基本用法（立即启动）
```python
from TTHSD_interface import TTHSDownloader

def on_progress(event, msg):
    if event['Type'] == 'update':
        total = msg.get('Total', 0)
        downloaded = msg.get('Downloaded', 0)
        task_id = event.get('ID', '')  # ID 从 event 获取
        if total:
            print(f"进度 [{task_id}]: {downloaded/total:.1%}")
    elif event['Type'] == 'startOne':
        url = msg.get('URL', '')
        task_id = event.get('ID', '')  # ID 从 event 获取
        index = msg.get('Index', 0)
        total = msg.get('Total', 0)
        print(f"开始下载 [{index}/{total}]: {url} (ID: {task_id})")
    elif event['Type'] == 'endOne':
        url = msg.get('URL', '')
        task_id = event.get('ID', '')  # ID 从 event 获取
        print(f"下载完成: {url} (ID: {task_id})")
    elif event['Type'] == 'err':
        print("错误:", msg.get('Error'))
    
    # 剩下的处理可以去查看 示例/TTHSD Python 接口封装 的 Callback 示例

with TTHSDownloader() as dl:
    dl_id = dl.start_download(
        urls=["https://example.com/file1.zip", "https://example.com/file2.zip"],
        save_paths=["./downloads/file1.zip", "./downloads/file2.zip"],
        thread_count=32,
        callback=on_progress
    )
    # 程序退出 with 块时自动 close()
```

### 4.2 创建后手动启动（并行下载）
```python
dl = TTHSDownloader()
dl_id = dl.get_downloader(
    urls=["https://example.com/bigfile.iso"],
    save_paths=["./downloads/bigfile.iso"],
    thread_count=64
)
dl.start_multiple_downloads_by_id(dl_id)
# ... 后续可暂停、恢复等
# 资源会在 dl 对象被销毁时自动清理
```

---

## 5. 内存管理

### 5.1 回调函数引用管理
每次调用 ***start_download*** 或 ***get_downloader*** 时，若提供了回调函数，会通过 ***ctypes.CFUNCTYPE*** 创建一个 C 可调用对象。该对象必须保持存活，否则 C 代码调用时会崩溃。

Python 接口通过以下方式管理这些引用：
- 回调对象被保存在 ***self._callback_refs*** 字典中，以回调对象的 id 为键
- 这些引用会随着 ***TTHSDownloader*** 实例一起被 Python 垃圾回收器自动清理
- 无需手动干预或调用额外的清理方法

---

## 6. 日志系统（内部）

模块内置了异步、线程安全的日志器 ***_TIlog***，日志默认：
- 输出到控制台
- 写入文件 ***TTHSDPyInter.log***（位于可执行文件目录或当前目录）
- 日志级别：***INFO***

可通过修改 ***_TIlog*** 的配置（不推荐，仅供调试）调整。日志文件采用 ***'cf'*** 模式（自动创建文件）。

**注意**：当前实现不包含 atexit 清理机制，日志会在程序正常退出时完成写入。

---

## 7. 注意事项

1. **DLL 路径**：若自动猜测失败，需显式传入正确路径。
2. **任务数据一致性**：***urls*** 与 ***save_paths*** 长度必须相等。
3. **ID 有效性**：所有控制方法（***pause*** 等）在 ID 不存在时返回 ***False***，不会抛出异常。
4. **回调异常**：回调中若抛出异常，会被模块捕获并记录到日志，但不会中断下载流程。
5. **多线程环境**：不同线程使用不同的 ***TTHSDownloader*** 实例是安全的。同一实例的方法（除回调注册外）在 Python 层面未加锁，但 DLL 内部状态由自身管理。

---

## 8. TTHSD Next 的优势

相比 TTHSD Golang 版本，使用 TTHSD Next 的 Python 接口可以获得以下优势：

### 性能提升
- **更高的下载速度**：优化的异步 I/O 模型
- **更低的内存占用**：精细的内存管理，无 GC 开销
- **更高的并发数**：可支持数十万并发连接
- **更稳定的性能**：无 GC 暂停，性能可预测

### 兼容性
- **完全兼容**：API 接口与 TTHSD Golang 版本完全兼容
- **直接替换**：只需替换动态库文件即可迁移
- **代码无需修改**：现有代码可以直接使用

---

## 9. 常见问题

**Q: 为什么我的回调没有收到 ***update*** 事件？**  
A: 检查 ***callback*** 参数是否传入，且您的回调处理正常。也可通过日志观察是否有回调异常。

**Q: 一定需要手动调用 close() 方法吗？**  
A: 可以不需要。当前实现会自动管理资源，无需手动调用 close() 方法。Python 的垃圾回收机制会自动清理相关引用，但是如果想要调用也可以调用。

**Q: 需要并发下载多个任务组怎么办？**  
A: 创建多个 ***TTHSDownloader*** 实例，每个管理一组任务。它们独立运行。或配合 ***threading*** 模块执行多次 开始下载

**Q: TTHSD Next 与 TTHSD Golang 有什么区别？**  
A: TTHSD Next 是 TTHSD Golang 的 Rust 重写版本，具有更高的性能、更低的内存占用和更稳定的运行表现。API 接口完全兼容，只需替换动态库即可迁移。

**Q: 我可以从 TTHSD Golang 迁移到 TTHSD Next 吗？**  
A: 是的，API 接口完全兼容，只需将动态库文件从 TTHighSpeedDownloader.dll/so/dylib 替换为 TTHSD.dll/so/dylib 即可，代码无需修改。

---

如有问题，请参考 [TTHSD Next GitHub](https://github.com/sxxyrry/TTHSDNext) 或提交 issue。
