"""Unicode 文件名兼容性测试 — 需要 DLL 和测试服务器。"""

from __future__ import annotations

import time
from pathlib import Path

import pytest


@pytest.mark.integration
class TestUnicodeFilename:
    def test_chinese_filename(self, tld_instance, base_url, tmp_download_dir):
        url = f"{base_url}/tiny_1kb.bin"
        save_path = str(tmp_download_dir / "下载测试_文件名.bin")

        dl = tld_instance._dl
        dl_id = dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
        )
        assert dl_id > 0
        time.sleep(3)

        filepath = Path(save_path)
        assert filepath.exists(), "中文文件名下载文件不存在"
        assert filepath.stat().st_size > 0, "中文文件名下载文件为空"

    def test_japanese_filename(self, tld_instance, base_url, tmp_download_dir):
        url = f"{base_url}/tiny_1kb.bin"
        save_path = str(tmp_download_dir / "日本語のテスト.bin")

        dl = tld_instance._dl
        dl_id = dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
        )
        assert dl_id > 0
        time.sleep(3)

        filepath = Path(save_path)
        assert filepath.exists(), "日文文件名下载文件不存在"

    def test_special_chars_filename(self, tld_instance, base_url, tmp_download_dir):
        url = f"{base_url}/tiny_1kb.bin"
        save_path = str(tmp_download_dir / "test (1) [test] - 副本.bin")

        dl = tld_instance._dl
        dl_id = dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
        )
        assert dl_id > 0
        time.sleep(3)

        filepath = Path(save_path)
        assert filepath.exists(), "特殊字符文件名下载文件不存在"
