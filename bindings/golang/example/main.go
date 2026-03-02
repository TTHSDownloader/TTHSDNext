// Package main 是 TTHSD Go 绑定的使用示例
package main

import (
	"fmt"
	"os"
	"os/signal"
	"syscall"

	tthsd "github.com/TTHSDownloader/TTHSDNext/bindings/golang"
)

func main() {
	// 1. 加载动态库（空字符串自动搜索）
	dl, err := tthsd.Load("")
	if err != nil {
		fmt.Fprintf(os.Stderr, "加载失败: %v\n", err)
		os.Exit(1)
	}
	defer dl.Close()

	// 2. 启动下载
	id, events, err := dl.StartDownload(
		[]string{"https://example.com/file.zip"},
		[]string{"./file.zip"},
		tthsd.DownloadOptions{
			ThreadCount: 32,
			ChunkSizeMB: 10,
		},
	)
	if err != nil {
		fmt.Fprintf(os.Stderr, "启动失败: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("下载 ID: %d\n", id)

	// 3. 监听中断信号
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)

	// 4. 处理事件
	for {
		select {
		case evt, ok := <-events:
			if !ok {
				fmt.Println("事件 channel 已关闭")
				return
			}

			switch evt.Event.Type {
			case tthsd.EventUpdate:
				downloaded, _ := evt.Data["Downloaded"].(float64)
				total, _ := evt.Data["Total"].(float64)
				if total > 0 {
					pct := downloaded / total * 100
					fmt.Printf("\r[%s] 进度: %.1f%%", evt.Event.ShowName, pct)
				}

			case tthsd.EventStartOne:
				fmt.Printf("\n▶ 开始下载: %s\n", evt.Event.ShowName)

			case tthsd.EventEndOne:
				fmt.Printf("\n✅ 完成: %s\n", evt.Event.ShowName)

			case tthsd.EventEnd:
				fmt.Println("\n🏁 全部下载完成")
				dl.StopDownload(id)
				return

			case tthsd.EventErr:
				errMsg, _ := evt.Data["Error"].(string)
				fmt.Fprintf(os.Stderr, "\n❌ 错误: %s\n", errMsg)
				dl.StopDownload(id)
				return

			case tthsd.EventMsg:
				text, _ := evt.Data["Text"].(string)
				fmt.Printf("\n📢 %s\n", text)
			}

		case <-sigCh:
			fmt.Println("\n中断，正在停止下载...")
			dl.StopDownload(id)
			return
		}
	}
}
