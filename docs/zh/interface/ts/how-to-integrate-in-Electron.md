# TTHSDownloader TypeScript 库 Electron 集成指南

本文档指导您将 **TTHSDownloader** TypeScript 库（基于 ***ffi-napi*** 调用 TT 高速下载器动态库）集成到 Electron 应用中。该库可通过 npm 包 ***tthsd-interface*** 安装，并用于在 Electron 主进程中创建、控制多任务下载，通过回调接收进度事件。

> [!TIP]
> 本文档介绍的是 **TTHSD Next**（Rust 版本）的集成方式。
> 
> API 接口与 [TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 版本完全兼容，只需替换动态库即可迁移。
> 
> TTHSD Golang 已停止开发，建议使用 TTHSD Next 以获得更好的性能。

---

## 1. 概述

**TTHSDownloader** 库通过 Node.js 的 ***ffi-napi*** 直接调用 C 动态库（***.dll***/***.dylib***/***.so***），实现高性能下载功能。在 Electron 中，建议将该库运行于**主进程**，因为它依赖于原生模块，且需要与文件系统交互。通过 IPC（进程间通信），您可以安全地将下载进度和控制能力暴露给渲染进程。

---

## 2. 前提条件

- 已初始化的 Electron 项目（可使用 ***electron-forge***、***electron-builder*** 或手动搭建）
- Node.js 16+ 和 npm/yarn/pnpm
- TT 高速下载器动态库文件：
  - Windows: ***TTHSD.dll***
  - macOS: ***TTHSD.dylib***
  - Linux: ***TTHSD.so***
- 动态库架构（32/64 位）必须与 Electron 应用的架构一致

> [!TIP]
> 您可以在 [TTHSD Next GitHub Releases](https://github.com/sxxyrry/TTHSDNext/releases/latest) 页面下载最新版本的动态库文件。

---

## 3. 安装依赖

在 Electron 项目根目录执行：

```bash
npm install tthsd-interface
npm install --save-dev electron-rebuild
```

***tthsd-interface*** 已内置所需的 ***ffi-napi*** 和 ***ref-napi*** 依赖，并提供了 TypeScript 类型定义，因此无需单独安装这些包。

如果使用 yarn 或 pnpm，请相应替换命令。

---

## 4. 配置 TypeScript

创建或修改 ***tsconfig.json***：

```json
{
  "compilerOptions": {
    "target": "es6",
    "module": "commonjs",
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "types": ["node"]
  },
  "include": ["src/**/*"]
}
```

**注意**：由于 ***tthsd-interface*** 已提供类型定义，您无需手动为 ***ffi-napi*** 编写声明文件。***skipLibCheck: true*** 可避免潜在的库类型冲突。

---

## 5. 处理动态库路径

在开发环境和打包后，动态库的路径会不同。建议在项目中创建 ***resources*** 目录，将动态库文件放入其中。然后在代码中动态获取绝对路径。

**示例：获取动态库路径的工具函数**

```typescript
// src/utils/paths.ts
import { app } from 'electron';
import path from 'path';

export function getLibraryPath(): string {
    const isPackaged = app.isPackaged;
    const platform = process.platform;

    let libName: string;
    switch (platform) {
        case 'win32':
            libName = 'TTHSD.dll';
            break;
        case 'darwin':
            libName = 'TTHSD.dylib';
            break;
        case 'linux':
            libName = 'TTHSD.so';
            break;
        default:
            throw new Error(***Unsupported platform: ${platform}***);
    }

    if (isPackaged) {
        // 打包后动态库位于 resources 目录下
        return path.join(process.resourcesPath, libName);
    } else {
        // 开发时动态库位于项目根目录的 resources 文件夹
        return path.join(__dirname, '../../resources', libName);
    }
}
```

**注意**：上述路径需根据您的项目结构调整。

---

## 6. 在主进程中使用 TTHSDownloader

### 6.1 引入库并创建实例

```typescript
// src/main.ts
import { app, BrowserWindow, ipcMain } from 'electron';
import { TTHSDownloader } from 'tthsd-interface';  // 从已安装的包导入
import { getLibraryPath } from './utils/paths';

let mainWindow: BrowserWindow;
let downloader: TTHSDownloader;

app.whenReady().then(() => {
    // 初始化下载器（传入动态库路径）
    const libPath = getLibraryPath();
    downloader = new TTHSDownloader(libPath);

    createWindow();
});

function createWindow() {
    mainWindow = new BrowserWindow({
        width: 800,
        height: 600,
        webPreferences: {
            preload: path.join(__dirname, 'preload.js'), // 预加载脚本
            contextIsolation: true, // 启用上下文隔离，增强安全性
        },
    });
    mainWindow.loadFile('index.html');
}
```

### 6.2 通过 IPC 处理下载请求

在主进程中监听渲染进程发起的下载请求，并调用库方法。同时将下载进度通过 IPC 回传。

```typescript
ipcMain.handle('start-download', async (event, urls, savePaths) => {
    const id = downloader.start_download(
        urls,
        savePaths,
        64,              // 线程数
        10,              // 分块大小 (MB)
        (event, msg) => { // 回调函数
            // 通过 IPC 向渲染进程发送进度
            mainWindow.webContents.send('download-progress', { event, msg });
        },
        false,           // useCallbackUrl
        undefined,       // userAgent
        undefined,       // remoteCallbackUrl
        null,            // useSocket
        null             // isMultiple
    );
    return id;
});

ipcMain.handle('pause-download', (event, id) => {
    return downloader.pause_download(id);
});

ipcMain.handle('resume-download', (event, id) => {
    return downloader.resume_download(id);
});

ipcMain.handle('stop-download', (event, id) => {
    return downloader.stop_download(id);
});
```

---

## 7. 预加载脚本与 IPC 通信

为了安全地将功能暴露给渲染进程，需编写预加载脚本（***preload.ts***）：

```typescript
// src/preload.ts
import { contextBridge, ipcRenderer } from 'electron';

contextBridge.exposeInMainWorld('electronAPI', {
    start_download: (urls: string[], savePaths: string[]) =>
        ipcRenderer.invoke('start-download', urls, savePaths),
    pause_download: (id: number) => ipcRenderer.invoke('pause-download', id),
    resume_download: (id: number) => ipcRenderer.invoke('resume-download', id),
    stop_download: (id: number) => ipcRenderer.invoke('stop-download', id),
    onProgress: (callback: (event: any, msg: any) => void) => {
        ipcRenderer.on('download-progress', (_, data) => callback(data.event, data.msg));
    },
});
```

编译 TypeScript 预加载脚本（需在 ***tsconfig.json*** 中包含 ***preload.ts***，并单独输出到 ***dist***）。

---

## 8. 渲染进程调用示例

在渲染进程的 HTML 或 JavaScript 中：

```html
<!-- index.html -->
<button id="downloadBtn">开始下载</button>
<div id="progress"></div>

<script>
window.electronAPI.onProgress((event, msg) => {
    console.log('进度事件:', event);
    console.log('消息内容:', msg);
    if (event.Type === 'update') {
        const percent = (msg.Downloaded / msg.Total * 100).toFixed(1);
        document.getElementById('progress').innerText = ***进度: ${percent}%***;
    }
});

document.getElementById('downloadBtn').addEventListener('click', async () => {
    const id = await window.electronAPI.start_download(
        ['https://example.com/file.zip'],
        ['C:/Downloads/file.zip']  // Windows 路径示例，请根据平台调整
    );
    console.log('下载器ID:', id);
});
</script>
```

---

## 9. 打包应用

### 9.1 配置 electron-builder

在 ***package.json*** 中添加或修改 ***build*** 字段：

```json
{
  "build": {
    "files": [
      "dist/**/*",
      "node_modules/**/*"
    ],
    "extraResources": [
      {
        "from": "resources/",
        "to": "resources/",
        "filter": ["**/*"]
      }
    ]
  }
}
```

### 9.2 重新编译原生模块

由于 Electron 使用自己的 Node 运行时，必须重新编译 ***tthsd-interface*** 所依赖的 ***ffi-napi*** 和 ***ref-napi***。在 ***package.json*** 中添加脚本：

```json
"scripts": {
  "postinstall": "electron-rebuild"
}
```

然后运行 ***npm install*** 或 ***yarn*** 会自动触发重建。

如果手动重建，执行：

```bash
npx electron-rebuild
```

### 9.3 打包命令

```bash
npm run build    # 编译 TypeScript
npm run dist     # 打包应用（根据您的配置）
```

---

## 10. TTHSD Next 的优势

相比 TTHSD Golang 版本，在 Electron 中使用 TTHSD Next 可以获得以下优势：

### 性能提升
- **更高的下载速度**：优化的异步 I/O 模型
- **更低的内存占用**：精细的内存管理，无 GC 开销
- **更稳定的性能**：无 GC 暂停，性能可预测，对 Electron 主进程影响更小

### 兼容性
- **完全兼容**：API 接口与 TTHSD Golang 版本完全兼容
- **直接替换**：只需替换动态库文件即可迁移
- **代码无需修改**：现有代码可以直接使用

---

## 11. 常见问题

### Q1: 编译时提示找不到模块"tthsd-interface"的类型声明

**解决方法**：***tthsd-interface*** 已内置类型定义，通常无需额外操作。确保您的 ***tsconfig.json*** 中 ***moduleResolution*** 设置为 ***node***，并且 ***node_modules*** 被正确解析。如果问题仍然存在，请检查包版本是否支持您的 TypeScript 版本。

### Q2: 运行时错误"Dynamic Linking Error"或"无法加载 DLL"

- 检查动态库路径是否正确，尤其在打包后使用 ***process.resourcesPath***。
- 确认动态库架构与 Electron 匹配（例如均为 64 位）。
- 在 Windows 上，确保系统已安装必要的 VC++ 运行库。

### Q3: 回调函数未触发

- 确认 ***ffi.Callback*** 创建的引用未被垃圾回收。***tthsd-interface*** 内部已使用 Map 保存回调引用，通常安全。
- 检查动态库是否正确调用回调。

### Q4: 打包后找不到动态库

- 验证 ***extraResources*** 配置是否正确，动态库是否被复制到 ***resources*** 目录。
- 在代码中打印 ***process.resourcesPath*** 和最终路径，确认文件存在。

### Q5: 渲染进程无法调用 IPC 方法

- 确保预加载脚本正确编译并加载（在 ***BrowserWindow*** 中指定了正确的 ***preload*** 路径）。
- 检查 ***contextIsolation*** 是否启用，并正确使用 ***contextBridge***。

### Q6: 在不同平台上的路径处理

建议使用条件判断区分平台，如前面的 ***getLibraryPath*** 函数所示。

### Q7: TTHSD Next 与 TTHSD Golang 有什么区别？

A: TTHSD Next 是 TTHSD Golang 的 Rust 重写版本，具有更高的性能、更低的内存占用和更稳定的运行表现。API 接口完全兼容，只需替换动态库即可迁移。

---

## 12. 完整示例项目

您可以参考以下目录结构的最小示例：

```
my-electron-app/
├── resources/
│   ├── TTHSD.dll   (Windows)
│   ├── TTHSD.dylib (macOS)
│   └── TTHSD.so    (Linux)
├── src/
│   ├── utils/
│   │   └── paths.ts                (路径工具)
│   ├── main.ts                      (主进程)
│   ├── preload.ts                   (预加载脚本)
│   └── renderer/
│       └── index.html               (渲染页面)
├── package.json
├── tsconfig.json
└── ...
```

按照本文档配置后，您即可在 Electron 应用中享受 TT 高速下载器的强大功能。

---

如有更多问题，请查阅 [TTHSD Next GitHub](https://github.com/sxxyrry/TTHSDNext) 或 [tthsd-interface ( typescript ) 的 GitHub 主页](https://github.com/sxxyrry/tthsd-interface-ts)。祝集成顺利！
