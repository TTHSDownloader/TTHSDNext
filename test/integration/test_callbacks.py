"""回调事件完整性测试 — 需要 DLL 和本地测试服务器。"""

from __future__ import annotations

import time

import pytest


@pytest.mark.integration
class TestCallbackEvents:
    def test_callback_event_types(self, tld_instance, base_url, tmp_download_dir):
        events: list[dict] = []

        def callback(event, msg):
            events.append({"type": event.get("Type", ""), "event": event, "msg": msg})

        url = f"{base_url}/small_100kb.bin"
        save_path = str(tmp_download_dir / "callback_test.bin")

        dl = tld_instance._dl
        dl_id = dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
            chunk_size_mb=1,
            callback=callback,
        )
        assert dl_id > 0
        time.sleep(5)

        event_types = {e["type"] for e in events}
        assert "start" in event_types, "缺少 start 事件"
        assert "end" in event_types, "缺少 end 事件"
        assert "startOne" in event_types, "缺少 startOne 事件"
        assert "endOne" in event_types, "缺少 endOne 事件"

    def test_callback_includes_show_name(self, tld_instance, base_url, tmp_download_dir):
        events: list[dict] = []

        def callback(event, msg):
            events.append(event)

        url = f"{base_url}/tiny_1kb.bin"
        save_path = str(tmp_download_dir / "show_name_test.bin")

        dl = tld_instance._dl
        dl_id = dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
            callback=callback,
            show_names=["test_show_name"],
        )
        assert dl_id > 0
        time.sleep(3)

        start_one_events = [e for e in events if e.get("Type") == "startOne"]
        if start_one_events:
            assert start_one_events[0].get("ShowName") == "test_show_name"

    def test_no_error_events(self, tld_instance, base_url, tmp_download_dir):
        errors: list[str] = []

        def callback(event, msg):
            if event.get("Type") == "err":
                errors.append(msg.get("Error", "unknown"))

        url = f"{base_url}/tiny_1kb.bin"
        save_path = str(tmp_download_dir / "no_error_test.bin")

        dl = tld_instance._dl
        dl_id = dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
            callback=callback,
        )
        assert dl_id > 0
        time.sleep(3)

        assert len(errors) == 0, f"收到意外错误事件: {errors}"
