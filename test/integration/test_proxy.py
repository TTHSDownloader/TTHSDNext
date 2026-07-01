"""代理功能测试 — 需要 DLL。"""

from __future__ import annotations

import time

import pytest


@pytest.mark.integration
class TestProxy:
    def test_set_proxy(self, tld_instance, base_url, tmp_download_dir):
        url = f"{base_url}/tiny_1kb.bin"
        save_path = str(tmp_download_dir / "proxy_test.bin")

        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
        )
        assert dl_id > 0

        result = dl.set_proxy(dl_id, "http://proxy.example.com:8080")
        assert result, "设置代理失败"

        dl.stop_download(dl_id)

    def test_disable_proxy(self, tld_instance, base_url, tmp_download_dir):
        url = f"{base_url}/tiny_1kb.bin"
        save_path = str(tmp_download_dir / "no_proxy_test.bin")

        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
        )
        assert dl_id > 0

        result = dl.set_proxy(dl_id, None)
        assert result, "禁用代理失败"

        dl.stop_download(dl_id)
