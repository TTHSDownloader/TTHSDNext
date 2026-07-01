"""性能统计测试 — 需要 DLL。"""

from __future__ import annotations

import time

import pytest


@pytest.mark.integration
class TestPerformanceStats:
    def test_get_stats(self, tld_instance, base_url, tmp_download_dir):
        url = f"{base_url}/medium_1mb.bin"
        save_path = str(tmp_download_dir / "stats_test.bin")

        dl = tld_instance._dl
        dl_id = dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=4,
        )
        assert dl_id > 0
        time.sleep(2)

        stats = dl.get_performance_stats(dl_id)
        assert isinstance(stats, dict)

        if "total_bytes" in stats:
            assert stats["total_bytes"] >= 0, "total_bytes 应为非负数"

        dl.stop_download(dl_id)

    def test_stats_nonexistent_downloader(self, tld_instance):
        dl = tld_instance._dl
        stats = dl.get_performance_stats(99999)
        assert isinstance(stats, dict), "不存在的下载器应返回空字典"
