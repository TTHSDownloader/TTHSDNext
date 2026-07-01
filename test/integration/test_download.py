"""核心下载功能集成测试 — 需要 DLL 和本地测试服务器。"""

from __future__ import annotations

import time
from pathlib import Path

import pytest

from test.server.test_server import md5_file


@pytest.mark.integration
class TestSingleFileDownload:
    def test_small_file(self, tld_instance, base_url, tmp_download_dir, manifest):
        filename = "tiny_1kb.bin"
        url = f"{base_url}/{filename}"
        save_path = str(tmp_download_dir / filename)

        dl_id = tld_instance.start(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
            chunk_size_mb=1,
        )
        assert dl_id > 0, f"start_download 返回 {dl_id}"
        time.sleep(3)

        result_path = Path(save_path)
        assert result_path.exists(), "下载文件不存在"
        assert result_path.stat().st_size > 0, "下载文件为空"

    def test_md5_verification(self, tld_instance, base_url, tmp_download_dir, manifest):
        filename = "medium_1mb.bin"
        url = f"{base_url}/{filename}"
        save_path = str(tmp_download_dir / filename)

        dl_id = tld_instance.start(
            urls=[url],
            save_paths=[save_path],
            thread_count=4,
            chunk_size_mb=1,
        )
        assert dl_id > 0
        time.sleep(5)

        actual_md5 = md5_file(save_path)
        expected_md5 = str(manifest[filename]["md5"])
        assert actual_md5 == expected_md5, (
            f"MD5 不匹配: 期望={expected_md5[:12]}..., 实际={actual_md5[:12]}..."
        )


@pytest.mark.integration
class TestMultiFileDownload:
    def test_sequential_download(self, tld_instance, base_url, tmp_download_dir, manifest):
        files = ["tiny_1kb.bin", "small_100kb.bin", "medium_1mb.bin"]
        urls = [f"{base_url}/{f}" for f in files]
        save_paths = [str(tmp_download_dir / f) for f in files]

        dl_id = tld_instance.start(
            urls=urls,
            save_paths=save_paths,
            thread_count=4,
            chunk_size_mb=1,
        )
        assert dl_id > 0
        time.sleep(10)

        for filename in files:
            filepath = tmp_download_dir / filename
            assert filepath.exists(), f"{filename} 不存在"
            actual_md5 = md5_file(str(filepath))
            expected_md5 = str(manifest[filename]["md5"])
            assert actual_md5 == expected_md5, f"{filename} MD5 不匹配"


@pytest.mark.integration
class TestPauseResume:
    def test_pause_then_resume(self, tld_instance, base_url, tmp_download_dir):
        from scripts.tld_interface import TLDownloader

        def _collector(events):
            def cb(event, msg):
                events.append((event, msg))
            return cb

        url = f"{base_url}/large_10mb.bin"
        save_path = str(tmp_download_dir / "pause_resume.dat")

        dl = tld_instance._dl
        events = []
        dl_id = dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=8,
            chunk_size_mb=2,
            callback=_collector(events),
        )
        assert dl_id > 0
        time.sleep(2)

        paused = dl.pause_download(dl_id)
        assert paused, "暂停失败"
        time.sleep(1)

        resumed = dl.resume_download(dl_id)
        assert resumed, "恢复失败"
        time.sleep(2)

        dl.stop_download(dl_id)

    def test_stop_cleanup(self, tld_instance, base_url, tmp_download_dir):
        url = f"{base_url}/medium_1mb.bin"
        save_path = str(tmp_download_dir / "stop_test.dat")

        dl_id = tld_instance.start(
            urls=[url],
            save_paths=[save_path],
            thread_count=4,
        )
        assert dl_id > 0
        time.sleep(1)

        stopped = tld_instance._dl.stop_download(dl_id)
        assert stopped, "停止失败"


@pytest.mark.integration
class TestDeferredStart:
    def test_get_downloader_then_start(self, tld_instance, base_url, tmp_download_dir):
        url = f"{base_url}/small_100kb.bin"
        save_path = str(tmp_download_dir / "deferred.dat")

        dl_id = tld_instance.create(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
            chunk_size_mb=1,
        )
        assert dl_id > 0

        started = tld_instance._dl.start_download_by_id(dl_id)
        assert started, "start_download_by_id 失败"
        time.sleep(5)

        filepath = Path(save_path)
        assert filepath.exists() and filepath.stat().st_size > 0
