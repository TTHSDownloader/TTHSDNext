"""tld_interface.py Python 层逻辑单元测试 — 无需 DLL。"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

# conftest 已添加 scripts 到 sys.path
from scripts.tld_interface import _build_tasks_json, _default_dll_name


class TestBuildTasksJson:
    def test_basic(self):
        urls = ["https://example.com/file.zip"]
        paths = ["./downloads/file.zip"]
        result = _build_tasks_json(urls, paths)
        data = json.loads(result)
        assert len(data) == 1
        assert data[0]["url"] == "https://example.com/file.zip"
        assert data[0]["save_path"] == "./downloads/file.zip"

    def test_multiple_tasks(self):
        urls = ["https://a.com/1.zip", "https://b.com/2.zip"]
        paths = ["./dl/1.zip", "./dl/2.zip"]
        result = _build_tasks_json(urls, paths)
        data = json.loads(result)
        assert len(data) == 2

    def test_show_names(self):
        urls = ["https://example.com/file.zip"]
        paths = ["./dl/f.zip"]
        names = ["myfile"]
        result = _build_tasks_json(urls, paths, show_names=names)
        data = json.loads(result)
        assert data[0]["show_name"] == "myfile"

    def test_show_names_fallback(self):
        urls = ["https://example.com/file.zip"]
        paths = ["./dl/f.zip"]
        result = _build_tasks_json(urls, paths)
        data = json.loads(result)
        assert data[0]["show_name"] == "file.zip"

    def test_custom_ids(self):
        urls = ["https://example.com/a.zip"]
        paths = ["./dl/a.zip"]
        ids = ["custom-001"]
        result = _build_tasks_json(urls, paths, ids=ids)
        data = json.loads(result)
        assert data[0]["id"] == "custom-001"

    def test_mismatched_lengths(self):
        with pytest.raises(ValueError, match="长度不一致"):
            _build_tasks_json(
                ["https://a.com/1.zip"],
                ["./dl/1.zip", "./dl/2.zip"],
            )

    def test_task_headers(self):
        urls = ["https://example.com/file.zip"]
        paths = ["./dl/f.zip"]
        headers = [{"X-Custom": "value"}]
        result = _build_tasks_json(urls, paths, task_headers=headers)
        data = json.loads(result)
        assert data[0]["headers"] == {"X-Custom": "value"}

    def test_empty_urls(self):
        result = _build_tasks_json([], [])
        assert result == "[]"

    def test_unicode_url(self):
        urls = ["https://例子.测试/文件.zip"]
        paths = ["./dl/文件.zip"]
        result = _build_tasks_json(urls, paths)
        data = json.loads(result)
        assert "例子" in data[0]["url"]


class TestDefaultDllName:
    def test_returns_string(self):
        name = _default_dll_name()
        assert isinstance(name, str)
        assert name.endswith((".dll", ".so", ".dylib"))

    def test_contains_tailer_downloader(self):
        name = _default_dll_name()
        assert "TaiLerDownloader" in name

    def test_windows_dll(self, monkeypatch):
        monkeypatch.setattr(sys, "platform", "win32")
        monkeypatch.setattr(Path, "exists", lambda _: True)
        monkeypatch.setattr("platform.system", lambda: "Windows")
        monkeypatch.setattr("platform.machine", lambda: "AMD64")
        name = _default_dll_name()
        assert name.endswith(".dll")
