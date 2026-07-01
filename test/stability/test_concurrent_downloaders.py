"""并发多下载器稳定性测试 — 需要 DLL 和测试服务器。"""

from __future__ import annotations

import threading
import time
from pathlib import Path

import pytest


@pytest.mark.integration
class TestConcurrentDownloaders:
    def test_concurrent_three_downloaders(self, dll_path, base_url, tmp_download_dir):
        from scripts.tld_interface import TLDownloader

        files = ["tiny_1kb.bin", "small_100kb.bin", "medium_1mb.bin"]
        results: list[bool] = [False] * len(files)
        lock = threading.Lock()

        def download_one(filename: str, idx: int):
            url = f"{base_url}/{filename}"
            save_path = str(tmp_download_dir / f"concurrent_{idx}_{filename}")
            try:
                with TLDownloader(dll_path=dll_path) as dl:
                    dl_id = dl.start_download(
                        urls=[url],
                        save_paths=[save_path],
                        thread_count=2,
                    )
                    if dl_id > 0:
                        time.sleep(5)
                with lock:
                    results[idx] = Path(save_path).exists() and Path(save_path).stat().st_size > 0
            except Exception:
                with lock:
                    results[idx] = False

        threads = [
            threading.Thread(target=download_one, args=(f, i))
            for i, f in enumerate(files)
        ]

        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=30)

        assert all(results), f"并发下载结果: {results}"
