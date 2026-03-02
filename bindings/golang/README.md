# TTHSD Golang 封装

> 通过 CGo + `dlopen/dlsym` 在运行时动态加载 TTHSD 动态库，提供 Go 原生的 channel 事件流。

---

## 📁 文件结构

```
bindings/golang/
├── go.mod          # Go 模块定义
├── go.sum
├── event.go        # 事件类型定义（DownloadEvent, CallbackData 等）
├── native.go       # CGo 底层绑定（dlopen + C 函数指针调用包装器）
├── tthsd.go        # 高层封装（TTHSDownloader + channel 事件流）
└── example/
    └── main.go     # 完整使用示例
```

---

## 特性

- **CGo + dlopen**：运行时动态加载，无需链接时依赖
- **Go channel**：通过 `chan DownloadEventMsg` 接收事件，与 goroutine/select 完美配合
- **全局回调路由**：C 回调通过 `sync.RWMutex` 保护的全局 map 路由到对应 channel
- **跨平台**：自动选择 `dlopen` (Linux/macOS) 或 `LoadLibrary` (Windows)
- **信号安全**：示例演示了 SIGINT/SIGTERM 优雅停机

---

## 快速开始

```go
package main

import (
    "fmt"
    tthsd "github.com/TTHSDownloader/TTHSDNext/bindings/golang"
)

func main() {
    dl, err := tthsd.Load("")  // 自动搜索动态库
    if err != nil {
        panic(err)
    }
    defer dl.Close()

    id, events, err := dl.StartDownload(
        []string{"https://example.com/file.zip"},
        []string{"./file.zip"},
        tthsd.DownloadOptions{ThreadCount: 32},
    )
    if err != nil {
        panic(err)
    }

    fmt.Printf("下载 ID: %d\n", id)

    for evt := range events {
        switch evt.Event.Type {
        case tthsd.EventUpdate:
            downloaded, _ := evt.Data["Downloaded"].(float64)
            total, _ := evt.Data["Total"].(float64)
            fmt.Printf("\r进度: %.1f%%", downloaded/total*100)
        case tthsd.EventEnd:
            fmt.Println("\n下载完成")
            dl.StopDownload(id)
            return
        case tthsd.EventErr:
            fmt.Printf("\n错误: %v\n", evt.Data["Error"])
            dl.StopDownload(id)
            return
        }
    }
}
```

---

## API 参考

### `TTHSDownloader`

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `Load(path)` | `(*TTHSDownloader, error)` | 加载动态库 |
| `Close()` | — | 释放资源 |
| `StartDownload(urls, paths, opts)` | `(int, <-chan, error)` | 创建并启动 |
| `GetDownloader(urls, paths, opts)` | `(int, <-chan, error)` | 创建不启动 |
| `StartDownloadByID(id)` | `bool` | 顺序启动 |
| `StartMultipleDownloadsByID(id)` | `bool` | 并行启动 |
| `PauseDownload(id)` | `bool` | 暂停 |
| `ResumeDownload(id)` | `bool` | 恢复 |
| `StopDownload(id)` | `bool` | 停止（同时关闭 channel） |

### `DownloadOptions`

```go
type DownloadOptions struct {
    ThreadCount       int     // 默认 64
    ChunkSizeMB       int     // 默认 10
    UserAgent         *string
    UseCallbackURL    bool
    RemoteCallbackURL *string
    UseSocket         *bool
    IsMultiple        *bool
}
```

### 事件常量

```go
const (
    EventStart    = "start"
    EventStartOne = "startOne"
    EventUpdate   = "update"
    EventEnd      = "end"
    EventEndOne   = "endOne"
    EventMsg      = "msg"
    EventErr      = "err"
)
```

---

## 安装

```bash
go get github.com/TTHSDownloader/TTHSDNext/bindings/golang
```

将 `tthsd.dll` / `libtthsd.so` / `libtthsd.dylib` 放到可执行文件同级目录或系统库搜索路径中。
