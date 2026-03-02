# TTHSD Java / Kotlin 封装库

> 基于 JNA（桌面端）和 JNI（Android 端）调用 TTHSD 高速下载器。
> 适用于 Windows / Linux / macOS 桌面程序、Android 应用、Minecraft Mod/Plugin、第三方启动器等场景。

---

## 📁 文件结构

```
src/main/kotlin/com/tthsd/
├── TTHSDownloader.kt         # 高层封装类（用户直接使用）
├── TTHSDLibraryJNA.kt        # JNA 接口声明（桌面端）
├── TTHSDLibraryJNI.kt        # JNI 接口声明（Android 端）
├── TTHSDownloaderAndroid.kt  # Android 专用封装
└── NativeLibraryLoader.kt    # 动态库自动加载/提取工具
```

---

## 架构概览

```
┌──────────────────┐
│ TTHSDownloader   │  ← 用户使用的高层 API
├──────────────────┤
│ JNA (桌面端)     │  TTHSDLibraryJNA.kt
│ JNI (Android)    │  TTHSDLibraryJNI.kt
├──────────────────┤
│ tthsd.dll/so     │  ← Rust 编译的动态库
└──────────────────┘
```

- **桌面端**：通过 JNA 接口加载 `tthsd.dll` / `libtthsd.so` / `libtthsd.dylib`
- **Android**：通过 JNI 接口调用 `libtthsd.so`（对应 Rust 的 `android_export.rs`）

---

## 快速开始 (Kotlin)

```kotlin
val dl = TTHSDownloader()  // 自动从 JAR 提取或搜索动态库

val id = dl.startDownload(
    urls = listOf("https://example.com/a.zip"),
    savePaths = listOf("/tmp/a.zip"),
    threadCount = 32,
    callback = { event, data ->
        when (event.Type) {
            "update" -> {
                val pct = (data["Downloaded"] as Double) / (data["Total"] as Double) * 100
                print("\r进度: ${"%.1f".format(pct)}%")
            }
            "endOne" -> println("\n完成: ${event.ShowName}")
            "err" -> println("\n错误: ${data["Error"]}")
        }
    }
)

println("下载 ID: $id")
```

---

## API 参考

### `TTHSDownloader`

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `startDownload(urls, savePaths, ...)` | `Int` | 创建并启动下载，返回下载器 ID |
| `getDownloader(urls, savePaths, ...)` | `Int` | 创建下载器（不启动） |
| `startDownloadById(id)` | `Boolean` | 顺序启动 |
| `startMultipleDownloadsById(id)` | `Boolean` | 并行启动 |
| `pauseDownload(id)` | `Boolean` | 暂停 |
| `resumeDownload(id)` | `Boolean` | 恢复 |
| `stopDownload(id)` | `Boolean` | 停止并销毁 |
| `close()` | — | 释放资源（`AutoCloseable`） |

### `startDownload` 完整参数

```kotlin
fun startDownload(
    urls: List<String>,
    savePaths: List<String>,
    threadCount: Int = 64,          // 下载线程数
    chunkSizeMB: Int = 10,          // 分块大小 MB
    callback: DownloadCallback?,    // 进度回调
    useCallbackUrl: Boolean = false,
    userAgent: String? = null,
    remoteCallbackUrl: String? = null,
    useSocket: Boolean? = null,
    isMultiple: Boolean? = null,    // true=并行, false=顺序
    showNames: List<String>? = null,
    ids: List<String>? = null
): Int
```

### 回调类型

```kotlin
data class DownloadEvent(val Type: String, val Name: String?, val ShowName: String?, val ID: String?)

typealias DownloadCallback = (event: DownloadEvent, data: Map<String, Any?>) -> Unit
```

---

## Gradle 依赖

```kotlin
// build.gradle.kts
dependencies {
    implementation("com.google.code.gson:gson:2.10+")
    implementation("net.java.dev.jna:jna:5.13+")
}
```

---

## Android 使用

Android 端使用 JNI 而非 JNA：

```kotlin
// Application.onCreate() 中初始化
TTHSDLibraryJNI.load()

// 使用 JNI 接口
val id = TTHSDLibraryJNI.startDownload(
    tasksJson,
    threadCount = 16,
    chunkSizeMB = 10,
    useCallbackUrl = true,
    callbackUrl = "ws://localhost:8080",
    useSocket = false,
    isMultiple = false
)
```

> **注意**：Android 端通过远程回调 URL（WebSocket/Socket）接收事件，不支持函数指针回调。

---

## GC 安全

封装类内部维护 `callbackRefs: MutableMap<Int, ProgressCallback>`，持有所有 JNA 回调引用。在 `stopDownload()` 或 `close()` 时释放。**务必在下载完成后调用 `stopDownload()` 或 `close()`**。
