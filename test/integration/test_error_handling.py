"""错误处理测试 — 需要 DLL。"""

from __future__ import annotations

import time

import pytest


@pytest.mark.integration
class TestErrorHandling:
    def test_invalid_url(self, tld_instance, tmp_download_dir):
        url = "https://this-domain-does-not-exist-12345.com/file.bin"
        save_path = str(tmp_download_dir / "error_test.bin")

        dl = tld_instance._dl
        dl_id = dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=4,
        )
        assert dl_id > 0
        time.sleep(3)
        dl.stop_download(dl_id)

    def test_404_error_callback(self, tld_instance, base_url, tmp_download_dir):
        errors: list[dict] = []

        def callback(event, msg):
            if event.get("Type") == "err":
                errors.append(msg)

        url = f"{base_url}/nonexistent_file.bin"
        save_path = str(tmp_download_dir / "404_test.bin")

        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
            callback=callback,
        )
        assert dl_id > 0
        dl.set_retry_config(dl_id, max_retries=0)
        dl.start_download_by_id(dl_id)
        time.sleep(3)

        assert len(errors) > 0, "404 应产生错误事件"

    def test_empty_urls(self, tld_instance):
        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=[],
            save_paths=[],
        )
        assert dl_id == -1, "空 URL 列表应返回 -1"
