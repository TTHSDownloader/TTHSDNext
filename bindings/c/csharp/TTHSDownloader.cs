using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using System.Collections.Generic;
using System.Threading.Channels;
using System.Threading.Tasks;
using System.IO;

namespace TTHSD
{
    // ------------------------------------------------------------------
    // 原始 P/Invoke 底层绑定
    // ------------------------------------------------------------------

    internal static class NativeMethods
    {
        private const string LibNameWin   = "TTHSD";
        private const string LibNameLinux = "TTHSD";
        private const string LibNameMac   = "TTHSD";

        /// <summary>C 回调委托类型</summary>
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void ProgressCallback(
            [MarshalAs(UnmanagedType.LPStr)] string? eventJson,
            [MarshalAs(UnmanagedType.LPStr)] string? dataJson);

        [DllImport(LibNameWin, EntryPoint = "start_download", CallingConvention = CallingConvention.Cdecl)]
        public static extern int StartDownload(
            [MarshalAs(UnmanagedType.LPStr)] string tasksData,
            int taskCount, int threadCount, int chunkSizeMB,
            ProgressCallback? callback,
            [MarshalAs(UnmanagedType.I1)] bool useCallbackUrl,
            [MarshalAs(UnmanagedType.LPStr)] string? userAgent,
            [MarshalAs(UnmanagedType.LPStr)] string? remoteCallbackUrl,
            IntPtr useSocket,      // bool* (可为 IntPtr.Zero)
            IntPtr isMultiple      // bool* (可为 IntPtr.Zero)
        );

        [DllImport(LibNameWin, EntryPoint = "get_downloader", CallingConvention = CallingConvention.Cdecl)]
        public static extern int GetDownloader(
            [MarshalAs(UnmanagedType.LPStr)] string tasksData,
            int taskCount, int threadCount, int chunkSizeMB,
            ProgressCallback? callback,
            [MarshalAs(UnmanagedType.I1)] bool useCallbackUrl,
            [MarshalAs(UnmanagedType.LPStr)] string? userAgent,
            [MarshalAs(UnmanagedType.LPStr)] string? remoteCallbackUrl,
            IntPtr useSocket
        );

        [DllImport(LibNameWin, EntryPoint = "start_download_id",           CallingConvention = CallingConvention.Cdecl)] public static extern int StartDownloadId(int id);
        [DllImport(LibNameWin, EntryPoint = "start_multiple_downloads_id", CallingConvention = CallingConvention.Cdecl)] public static extern int StartMultipleDownloadsId(int id);
        [DllImport(LibNameWin, EntryPoint = "pause_download",              CallingConvention = CallingConvention.Cdecl)] public static extern int PauseDownload(int id);
        [DllImport(LibNameWin, EntryPoint = "resume_download",             CallingConvention = CallingConvention.Cdecl)] public static extern int ResumeDownload(int id);
        [DllImport(LibNameWin, EntryPoint = "stop_download",               CallingConvention = CallingConvention.Cdecl)] public static extern int StopDownload(int id);
    }

    // ------------------------------------------------------------------
    // 事件类型
    // ------------------------------------------------------------------

    public record DownloadEvent(string Type, string Name, string ShowName, string ID);

    public class DownloadEventArgs : EventArgs
    {
        public DownloadEvent Event { get; init; }
        public Dictionary<string, JsonElement> Data { get; init; }
        public DownloadEventArgs(DownloadEvent evt, Dictionary<string, JsonElement> data)
        { Event = evt; Data = data; }
    }

    // ------------------------------------------------------------------
    // 高层封装类 TTHSDownloader
    // ------------------------------------------------------------------

    /// <summary>
    /// TTHSD 高速下载器 C# 封装。
    ///
    /// <para>支持 async/await 事件流、WPF/AvaloniaUI/Unity 线程友好回调。</para>
    ///
    /// <code>
    /// await using var dl = new TTHSDownloader();
    /// await foreach (var ev in dl.StartDownloadAsync(
    ///     new[] {"https://example.com/a.zip"},
    ///     new[] {"/tmp/a.zip"}))
    /// {
    ///     if (ev.Event.Type == "update")
    ///         Console.WriteLine($"进度: {ev.Data["Downloaded"]}/{ev.Data["Total"]}");
    /// }
    /// </code>
    /// </summary>
    public sealed class TTHSDownloader : IAsyncDisposable
    {
        // 持有委托引用，防止 GC 回收
        private readonly Dictionary<int, NativeMethods.ProgressCallback> _callbacks = new();

        // ------------------------------------------------------------------
        // 公开 API
        // ------------------------------------------------------------------

        /// <summary>
        /// 创建并立即启动下载，返回 (下载器ID, 异步事件流)。
        /// 支持 await foreach 迭代所有事件直到 "end" 或 "err"。
        /// </summary>
        public (int Id, IAsyncEnumerable<DownloadEventArgs> Events) StartDownload(
            IEnumerable<string> urls,
            IEnumerable<string> savePaths,
            int threadCount   = 64,
            int chunkSizeMB   = 10,
            bool isMultiple   = false
        ) {
            var (urlList, pathList) = Validate(urls, savePaths);
            var tasksJson = BuildTasksJson(urlList, pathList);
            var channel = Channel.CreateUnbounded<DownloadEventArgs>();

            var cb = MakeCallback(channel.Writer);

            // 分配 bool* 参数
            var isMultiplePtr = AllocBool(isMultiple);

            int id = NativeMethods.StartDownload(
                tasksJson, urlList.Count, threadCount, chunkSizeMB,
                cb, false, null, null, IntPtr.Zero, isMultiplePtr
            );

            FreeBool(isMultiplePtr);

            if (id == -1) throw new InvalidOperationException("[TTHSD] StartDownload 失败（返回 -1）");
            _callbacks[id] = cb;

            return (id, channel.Reader.ReadAllAsync());
        }

        /// <summary>创建下载器但不立即启动，返回 (下载器ID, 异步事件流)</summary>
        public (int Id, IAsyncEnumerable<DownloadEventArgs> Events) GetDownloader(
            IEnumerable<string> urls,
            IEnumerable<string> savePaths,
            int threadCount = 64,
            int chunkSizeMB = 10
        ) {
            var (urlList, pathList) = Validate(urls, savePaths);
            var tasksJson = BuildTasksJson(urlList, pathList);
            var channel = Channel.CreateUnbounded<DownloadEventArgs>();
            var cb = MakeCallback(channel.Writer);

            int id = NativeMethods.GetDownloader(
                tasksJson, urlList.Count, threadCount, chunkSizeMB,
                cb, false, null, null, IntPtr.Zero
            );

            if (id == -1) throw new InvalidOperationException("[TTHSD] GetDownloader 失败（返回 -1）");
            _callbacks[id] = cb;
            return (id, channel.Reader.ReadAllAsync());
        }

        public bool StartDownloadById(int id)          => NativeMethods.StartDownloadId(id)           == 0;
        public bool StartMultipleDownloadsById(int id) => NativeMethods.StartMultipleDownloadsId(id)  == 0;
        public bool PauseDownload(int id)              => NativeMethods.PauseDownload(id)              == 0;
        public bool ResumeDownload(int id)             => NativeMethods.ResumeDownload(id)             == 0;
        public bool StopDownload(int id) {
            var ret = NativeMethods.StopDownload(id) == 0;
            _callbacks.Remove(id);
            return ret;
        }

        public ValueTask DisposeAsync() {
            _callbacks.Clear();
            return ValueTask.CompletedTask;
        }

        // ------------------------------------------------------------------
        // 私有工具
        // ------------------------------------------------------------------

        private static (List<string> urls, List<string> paths) Validate(
            IEnumerable<string> urls, IEnumerable<string> savePaths)
        {
            var u = new List<string>(urls);
            var p = new List<string>(savePaths);
            if (u.Count != p.Count) throw new ArgumentException(
                $"[TTHSD] urls 与 savePaths 长度不一致: {u.Count} vs {p.Count}");
            return (u, p);
        }

        private static string BuildTasksJson(List<string> urls, List<string> savePaths)
        {
            var tasks = new List<object>(urls.Count);
            for (int i = 0; i < urls.Count; i++)
            {
                string showName = Path.GetFileName(urls[i].Split('?')[0]);
                tasks.Add(new {
                    url       = urls[i],
                    save_path = savePaths[i],
                    show_name = string.IsNullOrEmpty(showName) ? $"task_{i}" : showName,
                    id        = Guid.NewGuid().ToString()
                });
            }
            return JsonSerializer.Serialize(tasks);
        }

        private static NativeMethods.ProgressCallback MakeCallback(
            ChannelWriter<DownloadEventArgs> writer)
        {
            return (eventJson, dataJson) =>
            {
                try
                {
                    var evt  = JsonSerializer.Deserialize<DownloadEvent>(eventJson ?? "{}")!;
                    var data = JsonSerializer.Deserialize<Dictionary<string, JsonElement>>(dataJson ?? "{}")!;
                    var args = new DownloadEventArgs(evt, data);

                    // 非阻塞写入 channel
                    writer.TryWrite(args);

                    // 下载结束/出错时关闭 channel（让 await foreach 终止）
                    if (evt.Type is "end" or "err")
                        writer.TryComplete();
                }
                catch { /* 回调内异常不应崩溃 */ }
            };
        }

        private static IntPtr AllocBool(bool value)
        {
            IntPtr ptr = Marshal.AllocHGlobal(1);
            Marshal.WriteByte(ptr, value ? (byte)1 : (byte)0);
            return ptr;
        }

        private static void FreeBool(IntPtr ptr)
        {
            if (ptr != IntPtr.Zero) Marshal.FreeHGlobal(ptr);
        }
    }
}
