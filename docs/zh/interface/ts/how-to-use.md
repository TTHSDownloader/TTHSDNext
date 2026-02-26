# TT 高速下载器 Node.js/TypeScript 接口封装使用文档

> [!TIP]
> 本文档介绍的是 **TTHSD Next**（Rust 版本）的 TypeScript 接口。
> 
> API 接口与 [TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 版本完全兼容，只需替换动态库即可迁移。
> 
> TTHSD Golang 已停止开发，建议使用 TTHSD Next 以获得更好的性能。

本文档介绍 ***TTHSDownloader*** TypeScript 类，该类为 **TT高速下载器**（***TTHSD.dll***/***.so***/***.dylib***）提供 Node.js 封装，支持多任务下载的创建、控制与进度回调。该封装以 npm 包 ***tthsd-interface*** 形式发布。

---

## 0. 安装说明

### 0.1 环境要求
- Node.js 14 及以上版本（推荐使用 16+）
- TypeScript 4.0 及以上版本（如使用纯 JavaScript 也可）

### 0.2 安装依赖

```bash
npm install tthsd-interface
```

***tthsd-interface*** 已内置所需的 ***ffi-napi*** 和 ***ref-napi*** 依赖，并提供了完整的 TypeScript 类型定义，因此无需单独安装这些包。

### 0.3 动态库文件
将 TT 高速下载器动态库放置在项目可访问的路径：
- Windows 平台：***TTHSD.dll***
- macOS 平台：***TTHSD.dylib***
- Linux 平台：***TTHSD.so***

> [!TIP]
> 您可以在 [TTHSD Next GitHub Releases](https://github.com/sxxyrry/TTHSDNext/releases/latest) 页面下载最新版本的动态库文件。

---

## 1. 模块概述

- **核心类**：***TTHSDownloader***（默认导出）
- **依赖**：内部使用 ***ffi-napi***、***ref-napi*** 调用 C 动态库，您无需直接操作它们
- **功能**：
  - 创建下载器实例（立即启动或只创建）
  - 控制下载（开始、暂停、恢复、停止）
  - 通过回调函数接收下载进度、事件通知（***update***、***endOne***、***end***、***msg***、***err*** 等）
  - 内置简单文件日志（调试用）

---

## 2. ***TTHSDownloader*** 类

### 2.1 构造函数
```typescript
constructor(dllPath?: string)
```
- **参数** ***dllPath***：动态库路径（可选）。若不提供，根据操作系统自动猜测默认文件名（Windows: ***./TTHSD.dll***，macOS: ***./TTHSD.dylib***，Linux: ***./TTHSD.so***），路径为当前工作目录。

### 2.2 创建下载器
#### ***start_download(...) => number***
立即创建并启动下载器。
```typescript
start_download(
    urls: string[],
    savePaths: string[],
    threadCount: number = 64,
    chunkSizeMb: number = 10,
    callback?: UserCallback,
    useCallbackUrl: boolean = false,
    userAgent?: string,
    remoteCallbackUrl?: string,
    useSocket?: boolean | null,
    isMultiple?: boolean | null
): number
```
- **返回**：下载器实例 ID（正整数），失败时返回 -1。

#### ***get_downloader(...) => number***
仅创建下载器（不启动），返回实例 ID。参数同 ***start_download***（不含 ***isMultiple***）。

### 2.3 控制下载
所有控制方法均返回 ***boolean***，***true*** 表示操作成功（DLL 返回 0），***false*** 表示失败（如 ID 不存在）。
- ***startDownloadById(downloaderId: number): boolean***：开始顺序下载
- ***startMultipleDownloadsById(downloaderId: number): boolean***：开始并行下载（实验性）
- ***pause_download(downloaderId: number): boolean***：暂停下载
- ***resume_download(downloaderId: number): boolean***：恢复下载
- ***stop_download(downloaderId: number): boolean***：停止下载

### 2.4 资源管理
接口内部会自动管理回调函数引用，防止被垃圾回收。当 ***TTHSDownloader*** 实例被销毁时，所有关联的回调引用也会被释放。**无需手动调用清理方法**。

---

## 3. 回调函数

### 3.1 定义格式
用户提供的回调函数需接受两个任意对象参数（实际为解析后的 JSON）：
```typescript
type UserCallback = (event: any, msg: any) => void;
```
- **event**：包含事件类型、名称等元数据
- **msg**：携带具体数据（进度、结果、错误等）

### 3.2 事件类型与数据格式 

本部分与 [Event 文档](/zh/event/event-overview) 相同，在此不过多赘述

### 3.3 线程安全要求
- 回调函数在 **DLL 内部的工作线程** 中执行，但在 Node.js 中会被包装后进入事件循环，因此不会阻塞其他操作。
- **禁止在回调中执行耗时操作**（如磁盘写入、网络请求），也**禁止直接进行可能阻塞事件循环的操作**。
- 如果多个下载器共享同一个回调，需自行加锁保护共享数据（Node.js 是单线程，但回调可能交错，可使用原子操作或队列）。

---

## 4. 使用示例

### 4.1 基本用法（立即启动）
```typescript
import { TTHSDownloader } from 'tthsd-interface';  // 从已安装的包导入

const onProgress = (event: any, msg: any) => {
    if (event.Type === 'update') {
        const total = msg.Total;
        const downloaded = msg.Downloaded;
        const taskId = event.ID; // ID 从 event 获取
        if (total) {
            console.log(***进度 [${taskId}]: ${(downloaded / total * 100).toFixed(1)}%***);
        }
    } else if (event.Type === 'startOne') {
        const url = msg.URL;
        const taskId = event.ID;
        console.log(***开始下载: ${url} (ID: ${taskId})***);
    } else if (event.Type === 'endOne') {
        const url = msg.URL;
        const taskId = event.ID;
        console.log(***下载完成: ${url} (ID: ${taskId})***);
    } else if (event.Type === 'err') {
        console.error('错误:', msg.Error);
    }
};

const downloader = new TTHSDownloader(); // 自动猜测路径
const id = downloader.start_download(
    ['https://example.com/file1.zip', 'https://example.com/file2.zip'],
    ['./downloads/file1.zip', './downloads/file2.zip'],
    32,
    5,
    onProgress
);

if (id > 0) {
    console.log('下载已启动，ID:', id);
}
```

### 4.2 创建后手动启动（并行下载）
```typescript
import { TTHSDownloader } from 'tthsd-interface';

const downloader = new TTHSDownloader('./custom/path/libTTHSD.so');
const id = downloader.get_downloader(
    ['https://example.com/bigfile.iso'],
    ['./downloads/bigfile.iso'],
    64,
    10
);
if (id > 0) {
    downloader.startMultipleDownloadsById(id);
    // 稍后可以暂停、恢复等
    // downloader.pause_download(id);
}
```

---

## 5. 内存管理

### 5.1 回调函数引用管理
每次调用 ***start_download*** 或 ***get_downloader*** 时，若提供了回调函数，会通过 ***ffi.Callback*** 创建一个 C 可调用对象。该对象必须保持存活，否则 C 代码调用时会崩溃。

TypeScript 接口通过以下方式管理这些引用：
- 回调对象被保存在 ***callbackRefs*** Map 中，以回调对象的地址（或随机数）为键
- 这些引用会随着 ***TTHSDownloader*** 实例一起被垃圾回收器自动清理
- **无需手动干预**，也不需调用任何清理方法

---

## 6. 日志系统（内部）

模块内置了简单的文件日志器 ***downloaderLogger***，日志默认：
- 输出到控制台
- 写入文件 ***TTHSDPyInter.log***（位于当前工作目录）
- 日志级别：***INFO***

可通过修改源码中的 ***downloaderLogger*** 配置调整（不推荐，仅供调试）。日志文件采用 ***'cf'*** 模式（自动创建文件）。

---

## 7. 注意事项

1. **DLL 路径**：若自动猜测失败，需显式传入正确路径。动态库必须与 Node.js 进程架构（32/64 位）匹配。
2. **任务数据一致性**：***urls*** 与 ***savePaths*** 长度必须相等。
3. **ID 有效性**：所有控制方法（***pause_download*** 等）在 ID 不存在时返回 ***false***，不会抛出异常。
4. **回调异常**：回调中若抛出异常，会被模块捕获并记录到日志，但不会中断下载流程。
5. **多线程环境**：Node.js 是单线程事件循环，但回调可能来自不同线程，不过经过 ffi 处理后它们会被调度到事件循环中，因此无需额外加锁。但要注意不要在回调中执行同步耗时操作。

---

## 8. TTHSD Next 的优势

相比 TTHSD Golang 版本，使用 TTHSD Next 的 TypeScript 接口可以获得以下优势：

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

**Q: 需要手动释放资源或调用 close() 吗？**  
A: 不需要。当前实现会自动管理资源，垃圾回收时会自动释放回调引用。如果希望立即释放，可以设置下载器实例为 ***null*** 并等待 GC，但通常无需这么做。

**Q: 需要并发下载多个任务组怎么办？**  
A: 创建多个 ***TTHSDownloader*** 实例，每个管理一组任务。它们独立运行。或配合 ***worker_threads*** 模块执行多次开始下载。

**Q: TTHSD Next 与 TTHSD Golang 有什么区别？**  
A: TTHSD Next 是 TTHSD Golang 的 Rust 重写版本，具有更高的性能、更低的内存占用和更稳定的运行表现。API 接口完全兼容，只需替换动态库即可迁移。

**Q: 我可以从 TTHSD Golang 迁移到 TTHSD Next 吗？**  
A: 是的，API 接口完全兼容，只需将动态库文件从 TTHighSpeedDownloader.dll/so/dylib 替换为 TTHSD.dll/so/dylib 即可，代码无需修改。

**Q: 安装 ***tthsd-interface*** 后还需要单独安装 ***ffi-napi*** 吗？**  
A: 不需要。***tthsd-interface*** 已将其作为依赖包含，您只需安装这一个包即可。

---

如有问题，请参考 [TTHSD Next GitHub](https://github.com/sxxyrry/TTHSDNext) 或 [tthsd-interface ( typescript ) 的 GitHub 主页](https://github.com/sxxyrry/tthsd-interface-ts)。
