"""自定义 Headers 功能测试 — 需要 DLL。"""

from __future__ import annotations

import time

import pytest


@pytest.mark.integration
class TestGlobalHeaders:
    def test_set_global_headers(self, tld_instance, base_url, tmp_download_dir):
        url = f"{base_url}/tiny_1kb.bin"
        save_path = str(tmp_download_dir / "global_headers.bin")

        headers = {
            "X-Custom-Header": "custom-value",
            "X-Test-Header": "test-value",
        }

        dl = tld_instance._dl
        dl_id = dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
            headers=headers,
        )
        assert dl_id > 0
        time.sleep(3)
        dl.stop_download(dl_id)


@pytest.mark.integration
class TestTaskHeaders:
    def test_set_task_headers(self, tld_instance, base_url, tmp_download_dir):
        url = f"{base_url}/tiny_1kb.bin"
        save_path = str(tmp_download_dir / "task_headers.bin")

        task_headers = [{"X-Task-Header": "task-value"}]

        dl = tld_instance._dl
        dl_id = dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
            task_headers=task_headers,
        )
        assert dl_id > 0
        time.sleep(3)
        dl.stop_download(dl_id)


@pytest.mark.integration
class TestCombinedHeaders:
    def test_combined_headers(self, tld_instance, base_url, tmp_download_dir):
        urls = [f"{base_url}/tiny_1kb.bin", f"{base_url}/tiny_1kb.bin"]
        save_paths = [
            str(tmp_download_dir / "combined_1.bin"),
            str(tmp_download_dir / "combined_2.bin"),
        ]

        global_headers = {"X-Global": "global-value"}
        task_headers = [
            {"X-Task": "task-value-1"},
            {"X-Task": "task-value-2"},
        ]

        dl = tld_instance._dl
        dl_id = dl.start_download(
            urls=urls,
            save_paths=save_paths,
            thread_count=2,
            headers=global_headers,
            task_headers=task_headers,
        )
        assert dl_id > 0
        time.sleep(5)
        dl.stop_download(dl_id)
