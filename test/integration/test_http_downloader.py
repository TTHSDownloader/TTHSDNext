"""HTTP 下载器专项测试 — 需要 DLL 和本地测试服务器。

覆盖 HTTP 下载器的核心行为：Range 请求、分块下载、断点续传等。
"""

from __future__ import annotations

import time
from pathlib import Path

import pytest

from test.server.test_server import md5_file


@pytest.mark.integration
class TestHTTPBasicDownload:
    def test_single_file_download(self, tld_instance, base_url, tmp_download_dir, manifest):
        filename = "medium_1mb.bin"
        url = f"{base_url}/{filename}"
        save_path = str(tmp_download_dir / filename)

        dl_id = tld_instance._dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=4,
            chunk_size_mb=1,
        )
        assert dl_id > 0
        time.sleep(8)

        result_path = Path(save_path)
        assert result_path.exists()
        expected = str(manifest[filename]["md5"])
        actual = md5_file(save_path)
        assert actual == expected, f"MD5 不匹配: 期望={expected[:12]}..., 实际={actual[:12]}..."

    def test_multithreaded_download(self, tld_instance, base_url, tmp_download_dir, manifest):
        filename = "large_10mb.bin"
        url = f"{base_url}/{filename}"
        save_path = str(tmp_download_dir / filename)

        dl_id = tld_instance._dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=8,
            chunk_size_mb=2,
        )
        assert dl_id > 0
        time.sleep(15)

        result_path = Path(save_path)
        assert result_path.exists()
        expected = str(manifest[filename]["md5"])
        actual = md5_file(save_path)
        assert actual == expected, f"MD5 不匹配"

    def test_tiny_file_fast(self, tld_instance, base_url, tmp_download_dir, manifest):
        filename = "tiny_1kb.bin"
        url = f"{base_url}/{filename}"
        save_path = str(tmp_download_dir / filename)

        dl_id = tld_instance._dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
        )
        assert dl_id > 0
        time.sleep(3)

        expected = str(manifest[filename]["md5"])
        actual = md5_file(save_path)
        assert actual == expected


@pytest.mark.integration
class TestHTTPUserAgent:
    def test_custom_user_agent(self, tld_instance, base_url, tmp_download_dir):
        url = f"{base_url}/tiny_1kb.bin"
        save_path = str(tmp_download_dir / "ua_test.bin")

        dl = tld_instance._dl
        dl_id = dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
            user_agent="TLD-Test-Agent/1.0",
        )
        assert dl_id > 0
        time.sleep(3)

        filepath = Path(save_path)
        assert filepath.exists()


@pytest.mark.integration
class TestHTTPSequentialMulti:
    def test_three_files_sequential(self, tld_instance, base_url, tmp_download_dir, manifest):
        files = ["tiny_1kb.bin", "small_100kb.bin", "medium_1mb.bin"]
        urls = [f"{base_url}/{f}" for f in files]
        save_paths = [str(tmp_download_dir / f) for f in files]

        dl_id = tld_instance._dl.start_download(
            urls=urls,
            save_paths=save_paths,
            thread_count=4,
            chunk_size_mb=1,
        )
        assert dl_id > 0
        time.sleep(15)

        for filename in files:
            filepath = tmp_download_dir / filename
            assert filepath.exists(), f"{filename} 不存在"
            actual_md5 = md5_file(str(filepath))
            expected_md5 = str(manifest[filename]["md5"])
            assert actual_md5 == expected_md5


@pytest.mark.integration
class TestHTTPProgressEvents:
    def test_progress_received(self, tld_instance, base_url, tmp_download_dir):
        events: list[dict] = []

        def callback(event, msg):
            events.append({"type": event.get("Type", ""), "time": time.time()})

        url = f"{base_url}/medium_1mb.bin"
        save_path = str(tmp_download_dir / "progress_test.bin")

        dl = tld_instance._dl
        dl_id = dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=4,
            callback=callback,
        )
        assert dl_id > 0
        time.sleep(8)

        event_types = {e["type"] for e in events}
        assert "start" in event_types
        assert "update" in event_types, "HTTP 下载应收到 update 事件"
        assert "endOne" in event_types
        assert "end" in event_types

    def test_progress_values_increase(self, tld_instance, base_url, tmp_download_dir):
        updates: list[dict] = []

        def callback(event, msg):
            if event.get("Type") == "update":
                updates.append(msg)

        url = f"{base_url}/large_10mb.bin"
        save_path = str(tmp_download_dir / "progress_values.bin")

        dl = tld_instance._dl
        dl_id = dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=8,
            callback=callback,
        )
        assert dl_id > 0
        time.sleep(12)
        dl.stop_download(dl_id)

        if len(updates) >= 2:
            first_downloaded = updates[0].get("Downloaded", 0)
            last_downloaded = updates[-1].get("Downloaded", 0)
            assert last_downloaded >= first_downloaded, "下载进度应递增"


@pytest.mark.integration
class TestHTTPMultipleDownloaders:
    def test_two_independent_downloaders(self, dll_path, base_url, tmp_download_dir):
        from scripts.tld_interface import TLDownloader

        events_a: list = []
        events_b: list = []

        def cb_a(event, msg):
            events_a.append(event.get("Type"))

        def cb_b(event, msg):
            events_b.append(event.get("Type"))

        dl_a = TLDownloader(dll_path=dll_path)
        dl_b = TLDownloader(dll_path=dll_path)

        id_a = dl_a.start_download(
            urls=[f"{base_url}/small_100kb.bin"],
            save_paths=[str(tmp_download_dir / "multi_a.bin")],
            thread_count=2,
            callback=cb_a,
        )
        id_b = dl_b.start_download(
            urls=[f"{base_url}/small_100kb.bin"],
            save_paths=[str(tmp_download_dir / "multi_b.bin")],
            thread_count=2,
            callback=cb_b,
        )

        assert id_a > 0
        assert id_b > 0
        assert id_a != id_b, "两个下载器实例 ID 应该不同"

        time.sleep(6)

        dl_a.stop_download(id_a)
        dl_b.stop_download(id_b)
        dl_a.close()
        dl_b.close()

        assert "update" in events_a
        assert "update" in events_b
