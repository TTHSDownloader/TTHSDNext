"""速度限制功能测试 — 需要 DLL。"""

from __future__ import annotations

import time

import pytest


@pytest.mark.integration
class TestSpeedLimit:
    def test_set_speed_limit(self, tld_instance, base_url, tmp_download_dir):
        url = f"{base_url}/large_10mb.bin"
        save_path = str(tmp_download_dir / "speed_limit.bin")

        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=[url],
            save_paths=[save_path],
            thread_count=8,
            chunk_size_mb=2,
        )
        assert dl_id > 0

        result = dl.set_speed_limit(dl_id, 1024 * 1024)
        assert result, "设置速度限制失败"

        dl.start_download_by_id(dl_id)
        time.sleep(3)
        dl.stop_download(dl_id)

    def test_set_speed_limit_zero(self, tld_instance, base_url, tmp_download_dir):
        """0 表示不限速。"""
        url = f"{base_url}/medium_1mb.bin"
        save_path = str(tmp_download_dir / "no_limit.bin")

        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=[url],
            save_paths=[save_path],
            thread_count=4,
        )
        assert dl_id > 0

        result = dl.set_speed_limit(dl_id, 0)
        assert result, "设置不限速失败"

        dl.stop_download(dl_id)
