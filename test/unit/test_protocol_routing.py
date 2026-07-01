"""协议路由逻辑纯 Python 测试 — 模拟 Rust detect_scheme 逻辑，无需 DLL。

确保 Python 侧的协议路由理解与 Rust 侧 (get_downloader::detect_scheme) 一致。
"""

from __future__ import annotations

from enum import Enum, auto


class Protocol(Enum):
    Http = auto()
    Ftp = auto()
    Sftp = auto()
    BitTorrent = auto()
    Ed2k = auto()
    Metalink = auto()
    Unknown = auto()


def detect_scheme(url: str) -> Protocol:
    """镜像 Rust get_downloader::detect_scheme 的逻辑。"""
    lower = url.lower()
    if lower.startswith("http://") or lower.startswith("https://"):
        return Protocol.Http
    if lower.startswith("ftp://") or lower.startswith("ftps://"):
        return Protocol.Ftp
    if lower.startswith("sftp://"):
        return Protocol.Sftp
    if lower.startswith("magnet:") or lower.endswith(".torrent"):
        return Protocol.BitTorrent
    if lower.startswith("ed2k://"):
        return Protocol.Ed2k
    if lower.endswith(".metalink") or lower.endswith(".meta4"):
        return Protocol.Metalink
    return Protocol.Unknown


class TestProtocolRouting:
    def test_http(self):
        assert detect_scheme("http://example.com/file.zip") == Protocol.Http

    def test_https(self):
        assert detect_scheme("https://example.com/file.zip") == Protocol.Http

    def test_https_with_query(self):
        assert detect_scheme("https://cdn.example.com/dl?file=123&token=abc") == Protocol.Http

    def test_https_mixed_case(self):
        assert detect_scheme("HTTPS://EXAMPLE.COM/FILE.ZIP") == Protocol.Http

    def test_ftp(self):
        assert detect_scheme("ftp://ftp.gnu.org/README") == Protocol.Ftp

    def test_ftps(self):
        assert detect_scheme("ftps://secure-ftp.example.com/file") == Protocol.Ftp

    def test_sftp(self):
        assert detect_scheme("sftp://user@host:22/path/to/file") == Protocol.Sftp

    def test_sftp_with_password(self):
        assert detect_scheme("sftp://user:pass@host:22/file") == Protocol.Sftp

    def test_magnet(self):
        assert detect_scheme("magnet:?xt=urn:btih:ABC123&dn=test") == Protocol.BitTorrent

    def test_torrent_file(self):
        # Rust 中 http/https 优先级高于 .torrent 后缀检测
        assert detect_scheme("https://example.com/ubuntu.torrent") == Protocol.Http

    def test_torrent_file_with_query(self):
        assert detect_scheme("https://example.com/file.torrent?ref=1") == Protocol.Http

    def test_torrent_http(self):
        assert detect_scheme("http://example.com/file.torrent") == Protocol.Http

    def test_torrent_unknown_scheme(self):
        assert detect_scheme("unknown://example.com/file.torrent") == Protocol.BitTorrent

    def test_ed2k(self):
        assert detect_scheme("ed2k://|file|test.iso|1073741824|HASH|/") == Protocol.Ed2k

    def test_metalink(self):
        # Rust 中 http/https 优先级高于 .metalink 后缀检测
        assert detect_scheme("https://example.com/arch.metalink") == Protocol.Http

    def test_meta4(self):
        assert detect_scheme("https://example.com/arch.meta4") == Protocol.Http

    def test_metalink_http(self):
        assert detect_scheme("http://mirror.example.com/file.metalink") == Protocol.Http

    def test_metalink_unknown_scheme(self):
        assert detect_scheme("unknown://example.com/file.metalink") == Protocol.Metalink

    def test_unknown_scheme(self):
        assert detect_scheme("gopher://example.com/file") == Protocol.Unknown

    def test_empty_string(self):
        assert detect_scheme("") == Protocol.Unknown

    def test_random_text(self):
        assert detect_scheme("not a url at all") == Protocol.Unknown

    def test_ipfs(self):
        assert detect_scheme("ipfs://QmHash") == Protocol.Unknown

    def test_ipns(self):
        assert detect_scheme("ipns://example") == Protocol.Unknown

    def test_file_scheme(self):
        assert detect_scheme("file:///C:/path/to/file") == Protocol.Unknown

    def test_data_uri(self):
        assert detect_scheme("data:text/plain,hello") == Protocol.Unknown

    def test_thunder(self):
        assert detect_scheme("thunder://abc123") == Protocol.Unknown

    def test_magnet_case_insensitive(self):
        assert detect_scheme("MAGNET:?xt=urn:btih:ABC") == Protocol.BitTorrent

    def test_ed2k_case_insensitive(self):
        assert detect_scheme("ED2K://|file|t.iso|1|HASH|/") == Protocol.Ed2k

    def test_https_port(self):
        assert detect_scheme("https://example.com:8443/file.zip") == Protocol.Http

    def test_http_ip_address(self):
        assert detect_scheme("http://192.168.1.1/file.bin") == Protocol.Http

    def test_http_localhost(self):
        assert detect_scheme("http://127.0.0.1:8080/file.bin") == Protocol.Http

    def test_https_unicode_domain(self):
        assert detect_scheme("https://例子.测试/文件.zip") == Protocol.Http

    def test_unknown_long_url(self):
        assert detect_scheme(
            "custom-proto+scheme://very.long.domain.name/path/to/file?v=1&x=2"
        ) == Protocol.Unknown

    # ── 优先级测试：scheme 匹配优先于后缀匹配 ──

    def test_priority_http_over_torrent(self):
        """http/https scheme 优先级高于 .torrent 后缀。"""
        assert detect_scheme("http://example.com/file.torrent") == Protocol.Http
        assert detect_scheme("https://example.com/file.torrent") == Protocol.Http

    def test_priority_http_over_metalink(self):
        """http/https scheme 优先级高于 .metalink 后缀。"""
        assert detect_scheme("http://example.com/file.metalink") == Protocol.Http
        assert detect_scheme("https://example.com/file.meta4") == Protocol.Http
