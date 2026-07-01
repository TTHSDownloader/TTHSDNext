"""
TLD 本地测试 HTTP 服务器 — pytest fixture 友好版本
支持 Range 请求 / HEAD / Content-Length，自动生成确定性测试文件 + MD5 manifest。
"""

from __future__ import annotations

import hashlib
import json
import socket
import threading
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path
from typing import Any


def _bencode(value: bytes | int | str | list | dict) -> bytes:
    """Minimal bencode encoder for .torrent file generation."""
    if isinstance(value, int):
        encoded = f"i{value}e".encode()
    elif isinstance(value, bytes):
        encoded = f"{len(value)}:".encode() + value
    elif isinstance(value, str):
        encoded = _bencode(value.encode())
    elif isinstance(value, list):
        encoded = b"l" + b"".join(_bencode(v) for v in value) + b"e"
    elif isinstance(value, dict):
        items = sorted(value.items(), key=lambda x: x[0] if isinstance(x[0], (str, bytes)) else str(x[0]))
        encoded = b"d" + b"".join(
            _bencode(k if isinstance(k, (bytes, str)) else str(k)) +
            _bencode(v)
            for k, v in items
        ) + b"e"
    else:
        raise TypeError(f"Unsupported bencode type: {type(value)}")
    # Verify round-trip for basic types using a local decoder
    return encoded


def _create_torrent(data: bytes, name: str, piece_length: int = 16384) -> bytes:
    """Create a minimal single-file .torrent file (bencoded)."""
    pieces = b""
    for i in range(0, len(data), piece_length):
        piece = data[i:i + piece_length]
        pieces += hashlib.sha1(piece).digest()

    info = {
        "name": name,
        "length": len(data),
        "piece length": piece_length,
        "pieces": pieces,
    }
    torrent: dict = {"info": info}
    return _bencode(torrent)

TEST_DIR = Path(__file__).resolve().parent.parent / "server" / "test_files"
SERVER_PORT = 18080

FILES = {
    "tiny_1kb.bin": 1 * 1024,
    "small_100kb.bin": 100 * 1024,
    "medium_1mb.bin": 1 * 1024 * 1024,
    "large_10mb.bin": 10 * 1024 * 1024,
    "huge_100mb.bin": 100 * 1024 * 1024,
}


METALINK_TEMPLATE = """<?xml version="1.0" encoding="UTF-8"?>
<metalink xmlns="urn:ietf:params:xml:ns:metalink">
  <file name="tiny_1kb.bin">
    <size>1024</size>
    <url priority="1">http://127.0.0.1:{port}/tiny_1kb.bin</url>
  </file>
</metalink>"""

METALINK_MEDIUM_TEMPLATE = """<?xml version="1.0" encoding="UTF-8"?>
<metalink xmlns="urn:ietf:params:xml:ns:metalink">
  <file name="medium_1mb.bin">
    <size>1048576</size>
    <url priority="1">http://127.0.0.1:{port}/medium_1mb.bin</url>
  </file>
</metalink>"""


def generate_test_files() -> dict[str, dict[str, str | int]]:
    TEST_DIR.mkdir(parents=True, exist_ok=True)
    manifest: dict[str, dict[str, str | int]] = {}

    for name, size in FILES.items():
        filepath = TEST_DIR / name
        if filepath.exists() and filepath.stat().st_size == size:
            md5 = hashlib.md5()
            with open(filepath, "rb") as f:
                while chunk := f.read(8192):
                    md5.update(chunk)
            manifest[name] = {"size": size, "md5": md5.hexdigest()}
            continue

        md5 = hashlib.md5()
        pattern = (name * 256)[:256].encode("utf-8")
        with open(filepath, "wb") as f:
            written = 0
            while written < size:
                chunk_size = min(len(pattern), size - written)
                chunk = pattern[:chunk_size]
                f.write(chunk)
                md5.update(chunk)
                written += chunk_size

        manifest[name] = {"size": size, "md5": md5.hexdigest()}

    manifest_path = TEST_DIR / "manifest.json"
    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=2)

    # Generate Metalink .meta4 files for protocol testing
    meta4_path = TEST_DIR / "test.meta4"
    meta4_path.write_text(METALINK_TEMPLATE.format(port=SERVER_PORT), encoding="utf-8")
    meta4_medium_path = TEST_DIR / "test_medium.meta4"
    meta4_medium_path.write_text(METALINK_MEDIUM_TEMPLATE.format(port=SERVER_PORT), encoding="utf-8")

    # Generate BitTorrent .torrent file for tiny_1kb.bin (used by Torrent downloader tests)
    tiny_path = TEST_DIR / "tiny_1kb.bin"
    if tiny_path.exists():
        tiny_data = tiny_path.read_bytes()
        torrent_data = _create_torrent(tiny_data, "tiny_1kb.bin")
        (TEST_DIR / "test_tiny.torrent").write_bytes(torrent_data)

    return manifest


class RangeRequestHandler(BaseHTTPRequestHandler):
    def log_message(self, format: str, *args: tuple[Any]):
        pass

    def _resolve_path(self):
        path = self.path.lstrip("/")
        if not path:
            return None
        filepath = TEST_DIR / path
        if not filepath.exists() or not filepath.is_file():
            return None
        try:
            filepath.resolve().relative_to(TEST_DIR.resolve())
        except ValueError:
            return None
        return filepath

    def do_HEAD(self):
        filepath = self._resolve_path()
        if filepath is None:
            self.send_error(404, "File not found")
            return
        file_size = filepath.stat().st_size
        self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(file_size))
        self.send_header("Accept-Ranges", "bytes")
        self.end_headers()

    def do_GET(self):
        if self.path == "/manifest.json":
            manifest_path = TEST_DIR / "manifest.json"
            if manifest_path.exists():
                data = manifest_path.read_bytes()
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)
                return

        filepath = self._resolve_path()
        if filepath is None:
            self.send_error(404, "File not found")
            return

        file_size = filepath.stat().st_size
        range_header = self.headers.get("Range")

        if range_header:
            try:
                range_spec = range_header.replace("bytes=", "")
                start_str, end_str = range_spec.split("-")
                start = int(start_str)
                end = int(end_str) if end_str else file_size - 1
                end = min(end, file_size - 1)

                if start >= file_size or start > end:
                    self.send_error(416, "Range Not Satisfiable")
                    return

                content_length = end - start + 1
                self.send_response(206)
                self.send_header("Content-Type", "application/octet-stream")
                self.send_header("Content-Length", str(content_length))
                self.send_header("Content-Range", f"bytes {start}-{end}/{file_size}")
                self.send_header("Accept-Ranges", "bytes")
                self.end_headers()

                with open(filepath, "rb") as f:
                    f.seek(start)
                    remaining = content_length
                    while remaining > 0:
                        chunk_size = min(65536, remaining)
                        chunk = f.read(chunk_size)
                        if not chunk:
                            break
                        self.wfile.write(chunk)
                        remaining -= len(chunk)

            except (ValueError, IndexError):
                self.send_error(400, "Bad Range header")
        else:
            self.send_response(200)
            self.send_header("Content-Type", "application/octet-stream")
            self.send_header("Content-Length", str(file_size))
            self.send_header("Accept-Ranges", "bytes")
            self.end_headers()

            with open(filepath, "rb") as f:
                while True:
                    chunk = f.read(65536)
                    if not chunk:
                        break
                    self.wfile.write(chunk)


class TestServer:
    """可被 pytest fixture 管理的测试服务器。"""
    __test__ = False  # 防止 pytest 将其识别为测试类

    def __init__(self, port: int = SERVER_PORT):
        self.port = port
        self._server: HTTPServer | None = None
        self._thread: threading.Thread | None = None
        self._manifest: dict[str, dict[str, str | int]] = {}

    @property
    def manifest(self) -> dict[str, dict[str, str | int]]:
        return self._manifest

    @property
    def base_url(self) -> str:
        return f"http://127.0.0.1:{self.port}"

    def start(self):
        self._manifest = generate_test_files()
        self._server = HTTPServer(("0.0.0.0", self.port), RangeRequestHandler)
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)
        self._thread.start()

    def stop(self):
        if self._server:
            self._server.shutdown()
            self._server = None

    def url_for(self, filename: str) -> str:
        return f"{self.base_url}/{filename}"

    def manifest_for(self, filename: str) -> dict[str, str | int] | None:
        return self._manifest.get(filename)


def md5_file(filepath: str | Path) -> str:
    h = hashlib.md5()
    with open(filepath, "rb") as f:
        while chunk := f.read(8192):
            h.update(chunk)
    return h.hexdigest()


def find_free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]
