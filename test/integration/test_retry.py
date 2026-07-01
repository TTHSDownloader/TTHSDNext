"""重试配置测试 — 需要 DLL。"""

from __future__ import annotations

import time

import pytest


@pytest.mark.integration
class TestRetryConfig:
    def test_set_retry_config(self, tld_instance, base_url, tmp_download_dir):
        url = f"{base_url}/tiny_1kb.bin"
        save_path = str(tmp_download_dir / "retry_test.bin")

        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
        )
        assert dl_id > 0

        result = dl.set_retry_config(
            dl_id,
            max_retries=5,
            retry_delay_ms=2000,
            max_retry_delay_ms=60000,
        )
        assert result, "设置重试配置失败"

        dl.stop_download(dl_id)

    def test_default_retry_config(self, tld_instance, base_url, tmp_download_dir):
        url = f"{base_url}/tiny_1kb.bin"
        save_path = str(tmp_download_dir / "default_retry.bin")

        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
        )
        assert dl_id > 0

        result = dl.set_retry_config(dl_id)
        assert result, "设置默认重试配置失败"

        dl.stop_download(dl_id)
