package tthsd

import "encoding/json"

// DownloadEvent 是 DLL 回调中的事件结构（对应 Rust 中的 Event）
type DownloadEvent struct {
	Type     string `json:"Type"`
	Name     string `json:"Name"`
	ShowName string `json:"ShowName"`
	ID       string `json:"ID"`
}

// CallbackData 是回调中附带的数据
type CallbackData map[string]interface{}

// DownloadTask 是下载任务描述
type DownloadTask struct {
	URL      string `json:"url"`
	SavePath string `json:"save_path"`
	ShowName string `json:"show_name"`
	ID       string `json:"id"`
}

// EventType 常量
const (
	EventStart    = "start"    // 下载会话开始
	EventStartOne = "startOne" // 单个任务开始
	EventUpdate   = "update"   // 进度更新
	EventEnd      = "end"      // 全部任务完成
	EventEndOne   = "endOne"   // 单个任务完成
	EventMsg      = "msg"      // 消息通知
	EventErr      = "err"      // 错误
)

// DownloadEventMsg 是通过 channel 传递的事件消息
type DownloadEventMsg struct {
	Event DownloadEvent
	Data  CallbackData
}

// parseCallback 内部使用：解析 JSON 回调参数
func parseCallback(eventJSON, dataJSON string) (DownloadEvent, CallbackData) {
	var event DownloadEvent
	var data CallbackData

	if eventJSON != "" {
		_ = json.Unmarshal([]byte(eventJSON), &event)
	}
	if dataJSON != "" {
		_ = json.Unmarshal([]byte(dataJSON), &data)
	}

	return event, data
}
