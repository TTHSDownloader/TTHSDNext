package tthsd

/*
// goCallbackBridge 是从 C 回调到 Go 的桥接函数
// 声明在 native.go 的 CGo 头部中
*/
import "C"

import (
	"encoding/json"
	"fmt"
	"sync"

	"github.com/google/uuid"
)

// ---- 全局回调路由 ----
// 因为 C 回调不携带 userdata 指针，使用全局 map 做 ID -> channel 路由

var (
	callbackMu   sync.RWMutex
	callbackChans = make(map[int]chan DownloadEventMsg)
)

//export goCallbackBridge
func goCallbackBridge(eventJSON *C.char, dataJSON *C.char) {
	var eStr, dStr string
	if eventJSON != nil {
		eStr = C.GoString(eventJSON)
	}
	if dataJSON != nil {
		dStr = C.GoString(dataJSON)
	}

	event, data := parseCallback(eStr, dStr)
	msg := DownloadEventMsg{Event: event, Data: data}

	callbackMu.RLock()
	defer callbackMu.RUnlock()

	// 广播到所有已注册的 channel
	for _, ch := range callbackChans {
		select {
		case ch <- msg:
		default:
			// channel 满时丢弃（避免阻塞 C 回调线程）
		}
	}
}

func registerChannel(id int) <-chan DownloadEventMsg {
	ch := make(chan DownloadEventMsg, 1024)
	callbackMu.Lock()
	callbackChans[id] = ch
	callbackMu.Unlock()
	return ch
}

func unregisterChannel(id int) {
	callbackMu.Lock()
	if ch, ok := callbackChans[id]; ok {
		close(ch)
		delete(callbackChans, id)
	}
	callbackMu.Unlock()
}

// ---- DownloadOptions ----

// DownloadOptions 是 StartDownload / GetDownloader 的可选参数
type DownloadOptions struct {
	ThreadCount      int     // 下载线程数（默认 64）
	ChunkSizeMB      int     // 分块大小 MB（默认 10）
	UserAgent        *string // 自定义 UA（nil 使用内置默认值）
	UseCallbackURL   bool    // 是否启用远程回调
	RemoteCallbackURL *string // 远程回调地址
	UseSocket        *bool   // 是否使用 Socket
	IsMultiple       *bool   // 是否并行多任务
}

// DefaultOptions 返回默认下载选项
func DefaultOptions() DownloadOptions {
	return DownloadOptions{
		ThreadCount: 64,
		ChunkSizeMB: 10,
	}
}

// ---- TTHSDownloader ----

// TTHSDownloader 是 TTHSD 高速下载器的 Go 封装
type TTHSDownloader struct {
	lib *nativeLib
}

// Load 加载 TTHSD 动态库
//
// libPath 为动态库路径，空字符串则自动搜索默认文件名
// (tthsd.dll / libtthsd.so / libtthsd.dylib)
func Load(libPath string) (*TTHSDownloader, error) {
	lib, err := loadNativeLib(libPath)
	if err != nil {
		return nil, err
	}
	return &TTHSDownloader{lib: lib}, nil
}

// Close 释放动态库资源
func (dl *TTHSDownloader) Close() {
	if dl.lib != nil {
		dl.lib.close()
		dl.lib = nil
	}
}

// buildTasksJSON 构建任务列表 JSON
func buildTasksJSON(urls, savePaths []string, showNames, ids []string) (string, error) {
	if len(urls) != len(savePaths) {
		return "", fmt.Errorf("[TTHSD] urls 与 savePaths 长度不一致: %d vs %d",
			len(urls), len(savePaths))
	}

	tasks := make([]DownloadTask, len(urls))
	for i, url := range urls {
		showName := ""
		if showNames != nil && i < len(showNames) {
			showName = showNames[i]
		}
		if showName == "" {
			// 从 URL 中提取文件名
			for j := len(url) - 1; j >= 0; j-- {
				if url[j] == '/' {
					showName = url[j+1:]
					break
				}
			}
			// 去掉查询字符串
			for j := 0; j < len(showName); j++ {
				if showName[j] == '?' {
					showName = showName[:j]
					break
				}
			}
			if showName == "" {
				showName = fmt.Sprintf("task_%d", i)
			}
		}

		taskID := ""
		if ids != nil && i < len(ids) {
			taskID = ids[i]
		}
		if taskID == "" {
			taskID = uuid.New().String()
		}

		tasks[i] = DownloadTask{
			URL:      url,
			SavePath: savePaths[i],
			ShowName: showName,
			ID:       taskID,
		}
	}

	data, err := json.Marshal(tasks)
	if err != nil {
		return "", fmt.Errorf("[TTHSD] JSON 序列化失败: %w", err)
	}
	return string(data), nil
}

// StartDownload 创建并立即启动下载
//
// 返回 (下载器 ID, 事件 channel)。channel 会在下载结束/错误时自动关闭。
func (dl *TTHSDownloader) StartDownload(
	urls, savePaths []string,
	opts DownloadOptions,
) (int, <-chan DownloadEventMsg, error) {
	if dl.lib == nil {
		return -1, nil, fmt.Errorf("[TTHSD] 库未加载")
	}

	tasksJSON, err := buildTasksJSON(urls, savePaths, nil, nil)
	if err != nil {
		return -1, nil, err
	}

	threads := opts.ThreadCount
	if threads <= 0 {
		threads = 64
	}
	chunk := opts.ChunkSizeMB
	if chunk <= 0 {
		chunk = 10
	}

	id := dl.lib.callStartDownload(
		tasksJSON, len(urls), threads, chunk,
		opts.UseCallbackURL, opts.UserAgent, opts.RemoteCallbackURL,
		opts.UseSocket, opts.IsMultiple,
	)

	if id == -1 {
		return -1, nil, fmt.Errorf("[TTHSD] start_download 失败（返回 -1）")
	}

	ch := registerChannel(id)
	return id, ch, nil
}

// GetDownloader 创建下载器实例（不立即启动）
//
// 返回 (下载器 ID, 事件 channel)。之后通过 StartDownloadByID / StartMultipleDownloadsByID 启动。
func (dl *TTHSDownloader) GetDownloader(
	urls, savePaths []string,
	opts DownloadOptions,
) (int, <-chan DownloadEventMsg, error) {
	if dl.lib == nil {
		return -1, nil, fmt.Errorf("[TTHSD] 库未加载")
	}

	tasksJSON, err := buildTasksJSON(urls, savePaths, nil, nil)
	if err != nil {
		return -1, nil, err
	}

	threads := opts.ThreadCount
	if threads <= 0 {
		threads = 64
	}
	chunk := opts.ChunkSizeMB
	if chunk <= 0 {
		chunk = 10
	}

	id := dl.lib.callGetDownloader(
		tasksJSON, len(urls), threads, chunk,
		opts.UseCallbackURL, opts.UserAgent, opts.RemoteCallbackURL,
		opts.UseSocket,
	)

	if id == -1 {
		return -1, nil, fmt.Errorf("[TTHSD] get_downloader 失败（返回 -1）")
	}

	ch := registerChannel(id)
	return id, ch, nil
}

// StartDownloadByID 按 ID 顺序启动下载
func (dl *TTHSDownloader) StartDownloadByID(id int) bool {
	return dl.lib.callIntInt(dl.lib.fnStartDownloadID, id) == 0
}

// StartMultipleDownloadsByID 按 ID 并行启动下载
func (dl *TTHSDownloader) StartMultipleDownloadsByID(id int) bool {
	return dl.lib.callIntInt(dl.lib.fnStartMultipleDownloads, id) == 0
}

// PauseDownload 暂停下载
func (dl *TTHSDownloader) PauseDownload(id int) bool {
	return dl.lib.callIntInt(dl.lib.fnPauseDownload, id) == 0
}

// ResumeDownload 恢复下载（需核心版本 ≥0.5.1）
func (dl *TTHSDownloader) ResumeDownload(id int) bool {
	return dl.lib.callIntInt(dl.lib.fnResumeDownload, id) == 0
}

// StopDownload 停止并销毁下载器
func (dl *TTHSDownloader) StopDownload(id int) bool {
	ret := dl.lib.callIntInt(dl.lib.fnStopDownload, id) == 0
	unregisterChannel(id)
	return ret
}
