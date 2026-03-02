# TTHSD C# / .NET 封装

> P/Invoke 封装，支持 `async/await` 事件流，兼容 WPF / AvaloniaUI / Unity / MAUI。

---

## 📁 文件结构

| 文件 | 说明 |
|------|------|
| `TTHSDownloader.cs` | 核心封装类（包含 P/Invoke 声明和高层 API） |
| `example/Program.cs` | 控制台示例 |
| `example/TthsdExample.csproj` | .NET 项目文件 |

---

## 特性

- **async/await 事件流**：基于 `System.Threading.Channels`，支持 `await foreach` 遍历下载事件
- **IAsyncDisposable**：支持 `await using` 语法自动清理资源
- **GC 安全**：内部维护委托引用字典，防止 P/Invoke 回调被 GC 回收
- **.NET 6.0+**：使用 `System.Text.Json` 反序列化，无额外依赖

---

## 快速开始

```csharp
using TTHSD;

await using var dl = new TTHSDownloader();

var (id, events) = dl.StartDownload(
    new[] { "https://example.com/file.zip" },
    new[] { "./file.zip" },
    threadCount: 32
);

await foreach (var ev in events)
{
    switch (ev.Event.Type)
    {
        case "update":
            var downloaded = ev.Data["Downloaded"].GetInt64();
            var total = ev.Data["Total"].GetInt64();
            Console.Write($"\r进度: {downloaded * 100 / total}%");
            break;
        case "endOne":
            Console.WriteLine($"\n完成: {ev.Event.ShowName}");
            break;
        case "err":
            Console.WriteLine($"\n错误: {ev.Data["Error"]}");
            break;
    }
}
```

---

## API 参考

### `TTHSDownloader`

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `StartDownload(urls, paths, threadCount, chunkSizeMB, isMultiple)` | `(int Id, IAsyncEnumerable<DownloadEventArgs>)` | 创建并启动下载 |
| `GetDownloader(urls, paths, threadCount, chunkSizeMB)` | 同上 | 创建下载器（不启动） |
| `StartDownloadById(id)` | `bool` | 顺序启动 |
| `StartMultipleDownloadsById(id)` | `bool` | 并行启动 |
| `PauseDownload(id)` | `bool` | 暂停 |
| `ResumeDownload(id)` | `bool` | 恢复 |
| `StopDownload(id)` | `bool` | 停止并销毁 |

### 事件类型

```csharp
public record DownloadEvent(string Type, string Name, string ShowName, string ID);

public class DownloadEventArgs : EventArgs
{
    public DownloadEvent Event { get; }
    public Dictionary<string, JsonElement> Data { get; }
}
```

---

## 动态库放置

将 `tthsd.dll` / `libtthsd.so` / `libtthsd.dylib` 放到应用程序输出目录即可。P/Invoke 会自动搜索。

---

## 运行示例

```bash
cd example
dotnet run
```
