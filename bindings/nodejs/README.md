# tthsd (Node.js / TypeScript)

> TTHSD 高速下载器 Node.js / TypeScript 封装，支持 **Electron** 和纯 **Node.js** 环境。

---

## 📁 文件结构

```
src/
├── index.ts        # 统一导出入口
├── types.ts        # 完整的 TypeScript 类型定义
├── native.ts       # Koffi 底层绑定（动态库加载 + C ABI 映射）
└── downloader.ts   # TTHSDownloader 封装类（EventEmitter + 生命周期管理）
```

---

## 特性

- **Koffi FFI**：使用 [Koffi](https://koffi.dev/) 在运行时加载动态库，无需 C++ 编译 Node addon
- **Electron 兼容**：自动搜索 `app.asar.unpacked` 目录中的动态库
- **完整类型**：所有事件和参数均有 TypeScript 类型定义
- **EventEmitter**：继承自 Node.js `EventEmitter`
- **GC 安全**：通过 `koffi.register/unregister` 管理 C 回调引用

---

## 快速开始

```typescript
import { TTHSDownloader } from "tthsd";

const dl = new TTHSDownloader();
// 或指定动态库路径：new TTHSDownloader({ dllPath: "/opt/app/tthsd.so" })

const id = dl.startDownload(
  ["https://example.com/file.zip"],
  ["./file.zip"],
  {
    threadCount: 32,
    callback(event, data) {
      switch (event.Type) {
        case "update":
          const pct = ((data as any).Downloaded / (data as any).Total * 100).toFixed(1);
          process.stdout.write(`\r进度: ${pct}%`);
          break;
        case "endOne":
          console.log(`\n完成: ${event.ShowName}`);
          break;
        case "err":
          console.error(`\n错误: ${(data as any).Error}`);
          break;
      }
    },
  }
);

console.log(`下载 ID: ${id}`);
```

---

## API 参考

### `TTHSDownloader`

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `startDownload(urls, paths, options?)` | `number` | 创建并启动下载 |
| `getDownloader(urls, paths, options?)` | `number` | 创建下载器（不启动） |
| `startDownloadById(id)` | `boolean` | 顺序启动 |
| `startMultipleDownloadsById(id)` | `boolean` | 并行启动 |
| `pauseDownload(id)` | `boolean` | 暂停 |
| `resumeDownload(id)` | `boolean` | 恢复 |
| `stopDownload(id)` | `boolean` | 停止并销毁（同时释放 C 回调） |
| `dispose()` | `void` | 释放所有资源 |

### `DownloadOptions`

```typescript
interface DownloadOptions {
  threadCount?: number;       // 默认 64
  chunkSizeMB?: number;       // 默认 10
  callback?: DownloadCallback;
  userAgent?: string;
  useCallbackUrl?: boolean;
  remoteCallbackUrl?: string;
  useSocket?: boolean;
  isMultiple?: boolean;       // true=并行, false=顺序
  showNames?: string[];
  ids?: string[];
}
```

### 事件类型

```typescript
type EventType = "start" | "startOne" | "update" | "end" | "endOne" | "msg" | "err";

interface DownloadEvent {
  Type: EventType;
  Name: string;
  ShowName: string;
  ID: string;
}
```

---

## 动态库路径搜索

`TTHSDownloader` 会按以下顺序搜索动态库：

1. 用户通过 `dllPath` 参数指定的路径
2. Electron `app.asar.unpacked` 目录
3. 可执行文件同级目录
4. 当前工作目录 (`process.cwd()`)
5. `__dirname` 上级目录

---

## 安装

```bash
npm install koffi
# 将 tthsd.dll / libtthsd.so / libtthsd.dylib 放到项目根目录
```
