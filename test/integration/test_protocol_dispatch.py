"""协议分派集成测试 — 验证 C ABI 能正确为不同协议 URL 创建下载器。

需要 DLL。实际下载行为依赖对应协议服务器是否可达，
但创建下载器实例 (get_downloader) 应始终返回正数 ID。
"""

from __future__ import annotations

import time

import pytest


@pytest.mark.integration
class TestProtocolDispatch:
    """测试各种协议 URL 能否被路由到正确的下载器。"""

    def test_http_dispatch(self, tld_instance):
        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=["https://example.com/file.zip"],
            save_paths=["./http_test.zip"],
            thread_count=4,
        )
        assert dl_id > 0, f"HTTP URL 应返回正数 ID, 得到 {dl_id}"
        dl.stop_download(dl_id)

    def test_https_dispatch(self, tld_instance):
        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=["https://cdn.example.com/file.zip"],
            save_paths=["./https_test.zip"],
            thread_count=4,
        )
        assert dl_id > 0, f"HTTPS URL 应返回正数 ID, 得到 {dl_id}"
        dl.stop_download(dl_id)

    def test_ftp_dispatch(self, tld_instance):
        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=["ftp://ftp.gnu.org/README"],
            save_paths=["./ftp_test.txt"],
            thread_count=2,
        )
        assert dl_id > 0, f"FTP URL 应返回正数 ID, 得到 {dl_id}"
        dl.stop_download(dl_id)

    def test_ftps_dispatch(self, tld_instance):
        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=["ftps://secure.example.com/file"],
            save_paths=["./ftps_test.bin"],
            thread_count=2,
        )
        assert dl_id > 0, f"FTPS URL 应返回正数 ID, 得到 {dl_id}"
        dl.stop_download(dl_id)

    def test_sftp_dispatch(self, tld_instance):
        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=["sftp://user:pass@host:22/path/file"],
            save_paths=["./sftp_test.bin"],
            thread_count=2,
        )
        assert dl_id > 0, f"SFTP URL 应返回正数 ID, 得到 {dl_id}"
        dl.stop_download(dl_id)

    def test_magnet_dispatch(self, tld_instance):
        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=["magnet:?xt=urn:btih:ABC123&dn=test"],
            save_paths=["./magnet_test.iso"],
            thread_count=4,
        )
        assert dl_id > 0, f"Magnet URL 应返回正数 ID, 得到 {dl_id}"
        dl.stop_download(dl_id)

    def test_torrent_file_dispatch(self, tld_instance):
        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=["https://example.com/file.torrent"],
            save_paths=["./torrent_test.iso"],
            thread_count=4,
        )
        assert dl_id > 0, f"Torrent URL 应返回正数 ID, 得到 {dl_id}"
        dl.stop_download(dl_id)

    def test_ed2k_dispatch(self, tld_instance):
        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=["ed2k://|file|test.iso|1073741824|A1B2C3D4E5F6G7H8A1B2C3D4E5F6G7H8|/"],
            save_paths=["./ed2k_test.iso"],
            thread_count=2,
        )
        assert dl_id > 0, f"ED2K URL 应返回正数 ID, 得到 {dl_id}"
        dl.stop_download(dl_id)

    def test_metalink_dispatch(self, tld_instance):
        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=["https://example.com/arch.metalink"],
            save_paths=["./metalink_test.bin"],
            thread_count=4,
        )
        assert dl_id > 0, f"Metalink URL 应返回正数 ID, 得到 {dl_id}"
        dl.stop_download(dl_id)

    def test_meta4_dispatch(self, tld_instance):
        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=["https://example.com/arch.meta4"],
            save_paths=["./meta4_test.bin"],
            thread_count=4,
        )
        assert dl_id > 0, f"Meta4 URL 应返回正数 ID, 得到 {dl_id}"
        dl.stop_download(dl_id)


@pytest.mark.integration
class TestProtocolDispatchInvalid:
    """无效参数的协议分派测试。"""

    def test_empty_urls_returns_minus_one(self, tld_instance):
        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=[],
            save_paths=[],
        )
        assert dl_id == -1, f"空 URL 列表应返回 -1, 得到 {dl_id}"

    def test_unknown_protocol_falls_back(self, tld_instance):
        dl = tld_instance._dl
        dl_id = dl.get_downloader(
            urls=["gopher://example.com/file"],
            save_paths=["./unknown_test.bin"],
            thread_count=2,
        )
        assert dl_id > 0, f"未知协议应回退到 HTTP 并返回正数 ID, 得到 {dl_id}"
        dl.stop_download(dl_id)


@pytest.mark.integration
class TestProtocolStartDownload:
    """测试各协议启动下载（不要求服务器可达，只验证启动不崩溃）。"""

    @pytest.mark.parametrize("name,url,save", [
        ("http", "http://127.0.0.1:1/file.bin", "./dl_http.bin"),
        ("ftp", "ftp://127.0.0.1:1/file.bin", "./dl_ftp.bin"),
        ("sftp", "sftp://user@127.0.0.1:1/file.bin", "./dl_sftp.bin"),
        ("magnet", "magnet:?xt=urn:btih:AA&dn=test", "./dl_magnet.bin"),
        ("ed2k", "ed2k://|file|t.iso|1|A1B2C3D4E5F6G7H8A1B2C3D4E5F6G7H8|/", "./dl_ed2k.bin"),
    ])
    def test_start_download_no_server(self, name, url, save, tld_instance):
        dl = tld_instance._dl
        dl_id = dl.start_download(
            urls=[url],
            save_paths=[save],
            thread_count=2,
        )
        assert dl_id > 0, f"{name}: start_download 应返回正数 ID"
        time.sleep(2)
        dl.stop_download(dl_id)
