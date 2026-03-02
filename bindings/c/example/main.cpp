/**
 * C++ 调用 TTHSD 下载器示例
 *
 * 编译方式:
 *   mkdir build && cd build
 *   cmake .. && make
 *   cp /path/to/libtthsd.so ./
 *   ./download_example
 */

#include "../TTHSDownloader.hpp"
#include <iostream>
#include <atomic>
#include <thread>
#include <chrono>
#include <csignal>

std::atomic<bool> g_done{false};

int main() {
    TTHSDownloader dl;

    // 1. 加载动态库（空字符串 = 自动搜索当前目录）
    try {
        dl.load();
    } catch (const std::exception& e) {
        std::cerr << e.what() << "\n";
        return 1;
    }

    std::cout << "🚀 TTHSD C++ 示例启动\n";

    // 2. 启动下载，lambda 捕获回调事件
    int id = dl.startDownload(
        {"https://example.com/file.zip"},
        {"/tmp/file.zip"},
        DownloadParams{.threadCount = 32, .chunkSizeMB = 10},
        [](const json& event, const json& data) {
            std::string type = event.value("Type", "");
            std::string show = event.value("ShowName", "");

            if (type == "update") {
                int64_t downloaded = data.value("Downloaded", 0LL);
                int64_t total      = data.value("Total", 1LL);
                double  pct        = static_cast<double>(downloaded) / total * 100.0;
                printf("\r[%s] 进度: %lld/%lld (%.2f%%)",
                       show.c_str(), downloaded, total, pct);
                fflush(stdout);

            } else if (type == "startOne") {
                printf("\n▶ 开始 [%d/%d]: %s\n",
                       data.value("Index", 0),
                       data.value("Total", 0),
                       data.value("URL", "").c_str());

            } else if (type == "endOne") {
                printf("\n✅ 完成 [%d/%d]: %s\n",
                       data.value("Index", 0),
                       data.value("Total", 0),
                       data.value("URL", "").c_str());

            } else if (type == "end") {
                std::cout << "\n🏁 全部下载完成\n";
                g_done = true;

            } else if (type == "err") {
                std::cerr << "\n❌ 错误: " << data.value("Error", "未知") << "\n";
                g_done = true;
            }
        }
    );

    if (id == -1) {
        std::cerr << "startDownload 失败\n";
        return 1;
    }

    // 3. 等待下载结束
    while (!g_done) {
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
    }

    dl.stopDownload(id);
    return 0;
}
