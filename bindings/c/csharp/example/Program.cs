using System;
using System.IO;
using TTHSD;

namespace TthsdExample
{
    class Program
    {
        static async Task Main(string[] args)
        {
            Console.WriteLine("🚀 TTHSD C# 示例启动");

            // 动态库查找（P/Invoke 默认搜索路径，或将 TTHSD.dll/so/dylib 放到工作目录）
            await using var dl = new TTHSDownloader();

            var (id, events) = dl.StartDownload(
                new[] { "https://example.com/file.zip" },
                new[] { "/tmp/file.zip" },
                threadCount: 32,
                chunkSizeMB: 10
            );

            Console.WriteLine($"下载器 ID = {id}");

            // await foreach 异步接收事件（直到 "end" 或 "err" 信号 Channel 自动关闭）
            await foreach (var ev in events)
            {
                switch (ev.Event.Type)
                {
                    case "start":
                        Console.WriteLine("🚀 下载会话开始");
                        break;

                    case "startOne":
                        ev.Data.TryGetValue("Index", out var idxVal);
                        ev.Data.TryGetValue("Total", out var totVal);
                        ev.Data.TryGetValue("URL",   out var urlVal);
                        Console.WriteLine($"▶ 开始 [{idxVal}/{totVal}]: {urlVal}");
                        break;

                    case "update":
                        ev.Data.TryGetValue("Downloaded", out var dlVal);
                        ev.Data.TryGetValue("Total",      out var tlVal);
                        long downloaded = dlVal.TryGetInt64(out var d) ? d : 0;
                        long total      = tlVal.TryGetInt64(out var t) ? t : 1;
                        double pct = (double)downloaded / total * 100.0;
                        Console.Write($"\r进度: {downloaded}/{total} ({pct:F2}%)       ");
                        break;

                    case "endOne":
                        Console.WriteLine($"\n✅ 单文件完成: {ev.Event.ShowName}");
                        break;

                    case "end":
                        Console.WriteLine("\n🏁 全部下载完成");
                        break;

                    case "err":
                        ev.Data.TryGetValue("Error", out var errVal);
                        Console.Error.WriteLine($"\n❌ 错误: {errVal}");
                        break;

                    case "msg":
                        ev.Data.TryGetValue("Text", out var msgVal);
                        Console.WriteLine($"\n📢 消息: {msgVal}");
                        break;
                }
            }

            Console.WriteLine("程序结束");
        }
    }
}
