"""反复创建/销毁下载器稳定性测试 — 需要 DLL 和测试服务器。"""

from __future__ import annotations

import time
from pathlib import Path

import pytest


@pytest.mark.integration
class TestRepeatedCreateDestroy:
    @pytest.mark.parametrize("iteration", range(5))
    def test_repeated_create_destroy(
        self, iteration: int, dll_path, base_url, tmp_download_dir
    ):
        from scripts.tld_interface import TLDownloader

        url = f"{base_url}/tiny_1kb.bin"
        save_path = str(tmp_download_dir / f"repeat_{iteration}.bin")

        with TLDownloader(dll_path=dll_path) as dl:
            dl_id = dl.start_download(
                urls=[url],
                save_paths=[save_path],
                thread_count=2,
            )
            assert dl_id > 0
            time.sleep(2)

        filepath = Path(save_path)
        assert filepath.exists(), f"第 {iteration} 次迭代下载文件不存在"
