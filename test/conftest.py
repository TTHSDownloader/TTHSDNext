from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

import pytest

# 添加项目 scripts 和 test 目录到 path
PROJECT_ROOT = Path(__file__).resolve().parent.parent
SCRIPTS_DIR = PROJECT_ROOT / "scripts"
_TEST_DIR = Path(__file__).resolve().parent
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))
if str(_TEST_DIR) not in sys.path:
    sys.path.insert(0, str(_TEST_DIR))


@pytest.fixture(scope="session")
def project_root() -> Path:
    return PROJECT_ROOT


@pytest.fixture(scope="session")
def scripts_dir() -> Path:
    return SCRIPTS_DIR


@pytest.fixture(scope="session")
def dll_path() -> Path:
    """定位 TLD 动态库。默认在项目根目录下按操作系统名查找。"""
    if os.name == "nt":
        candidates = [
            PROJECT_ROOT / "TaiLerDownloader.dll",
            PROJECT_ROOT / "target/release/TaiLerDownloader.dll",
            PROJECT_ROOT / "target/debug/TaiLerDownloader.dll",
        ]
    elif sys.platform == "darwin":
        candidates = [
            PROJECT_ROOT / "TaiLerDownloader.dylib",
            PROJECT_ROOT / "TaiLerDownloader_arm64.dylib",
            PROJECT_ROOT / "target/release/TaiLerDownloader.dylib",
            PROJECT_ROOT / "target/release/TaiLerDownloader_arm64.dylib",
            PROJECT_ROOT / "target/debug/TaiLerDownloader.dylib",
            PROJECT_ROOT / "target/debug/TaiLerDownloader_arm64.dylib",
        ]
    else:
        candidates = [
            PROJECT_ROOT / "TaiLerDownloader.so",
            PROJECT_ROOT / "target/release/TaiLerDownloader.so",
            PROJECT_ROOT / "target/debug/TaiLerDownloader.so",
        ]

    for p in candidates:
        if p.exists():
            return p

    pytest.skip(f"未找到 TLD 动态库，尝试路径: {[str(c) for c in candidates]}")


@pytest.fixture
def tmp_download_dir() -> Path:
    """每个测试独立的临时下载目录，测试后自动清理。"""
    with tempfile.TemporaryDirectory(prefix="tld_test_") as tmp:
        yield Path(tmp)


@pytest.fixture(scope="session")
def tld_interface():
    """延迟导入 tld_interface，避免 session 级 import 失败。"""
    from scripts.tld_interface import TLDownloader, EventLogger
    return {"TLDownloader": TLDownloader, "EventLogger": EventLogger}


# ── 本地测试服务器（供 integration / stability 共用） ──

@pytest.fixture(scope="session")
def test_server_port() -> int:
    return 18080


@pytest.fixture(scope="session")
def test_server(test_server_port: int):
    from server.test_server import TestServer
    server = TestServer(port=test_server_port)
    server.start()
    yield server
    server.stop()


@pytest.fixture
def manifest(test_server) -> dict:
    return test_server.manifest


@pytest.fixture
def base_url(test_server) -> str:
    return test_server.base_url


@pytest.fixture
def tld_instance(dll_path: Path, tmp_download_dir: Path, request):
    from scripts.tld_interface import TLDownloader

    dl = TLDownloader(dll_path=dll_path)
    dl_ids: list[int] = []

    def _create(**kwargs):
        origin_urls = kwargs.pop("urls", [])
        if not origin_urls:
            origin_urls = [kwargs.pop("url", None)]
        origin_save_paths = kwargs.pop("save_paths", [])
        if not origin_save_paths:
            origin_save_paths = [str(tmp_download_dir / f"dl_{len(dl_ids)}.dat")]

        callback = kwargs.pop("callback", None)

        dl_id = dl.get_downloader(
            urls=origin_urls,
            save_paths=origin_save_paths,
            callback=callback,
            **kwargs,
        )
        dl_ids.append(dl_id)
        return dl_id

    def _start(**kwargs):
        dl_id = _create(**kwargs)
        dl.start_download_by_id(dl_id)
        return dl_id

    helper = type("Helper", (), {
        "create": staticmethod(_create),
        "start": staticmethod(_start),
        "_dl": dl,
        "_db_ids": dl_ids,
    })()

    yield helper

    for dl_id in dl_ids:
        try:
            dl.stop_download(dl_id)
        except Exception:
            pass
    dl.close()
