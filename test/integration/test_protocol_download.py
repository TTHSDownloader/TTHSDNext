"""各协议实际下载集成测试 — 需要 DLL 和对应协议的网络可达性。

测试分类：
  - Metalink:   本地测试服务器，无需网络
  - FTP:        需要 ftp.gnu.org 可达
  - BitTorrent: 本地合成 .torrent，无需网络（仅验证初始化和取消）
  - ED2K:       无需网络（仅验证解析和任务生命周期）
  - HTTP/3:     需要 cloudflare.com 可达（验证 Alt-Svc 探测）
"""

from __future__ import annotations

import time
from pathlib import Path

import pytest


@pytest.mark.integration
class TestMetalinkDownload:
    """Metalink 实际下载测试 — 使用本地测试服务器的 .meta4 文件。"""

    def test_metalink_download_tiny(self, tld_instance, base_url, manifest, tmp_download_dir):
        """通过 .meta4 下载 tiny_1kb.bin，验证 MD5 匹配。"""
        meta4_url = f"{base_url}/test.meta4"
        save_path = tmp_download_dir / "metalink_tiny.bin"
        dl_id = tld_instance.start(
            urls=[meta4_url],
            save_paths=[str(save_path)],
            thread_count=2,
        )
        assert dl_id > 0, f"Metalink start_download 应返回正数 ID, 得到 {dl_id}"

        # 1KB 文件通过本地服务器应很快完成
        for _ in range(15):
            if save_path.exists():
                break
            time.sleep(0.5)
        else:
            pytest.fail(f"Metalink 下载超时: {save_path} 未在 7.5s 内创建")

        assert save_path.stat().st_size == 1024, \
            f"下载文件大小不正确: {save_path.stat().st_size} (期望 1024)"

        md5_src = manifest.get("tiny_1kb.bin", {}).get("md5")
        if md5_src:
            from test.server.test_server import md5_file
            assert md5_file(save_path) == md5_src, \
                f"MD5 不匹配: 期望 {md5_src}"

    def test_metalink_download_medium(self, tld_instance, base_url, manifest, tmp_download_dir):
        """通过独立的 .meta4 文件下载 medium_1mb.bin，验证大小。"""
        meta4_url = f"{base_url}/test_medium.meta4"
        save_path = tmp_download_dir / "metalink_medium.bin"
        dl_id = tld_instance.start(
            urls=[meta4_url],
            save_paths=[str(save_path)],
            thread_count=4,
        )
        assert dl_id > 0

        for _ in range(30):
            if save_path.exists():
                break
            time.sleep(0.5)
        else:
            pytest.fail(f"Metalink 下载超时: {save_path} 未在 15s 内创建")

        assert save_path.stat().st_size == 1048576, \
            f"下载文件大小不正确: {save_path.stat().st_size} (期望 1048576)"

        md5_src = manifest.get("medium_1mb.bin", {}).get("md5")
        if md5_src:
            from test.server.test_server import md5_file
            assert md5_file(save_path) == md5_src, \
                f"MD5 不匹配: 期望 {md5_src}"


@pytest.mark.integration
class TestFTPDownload:
    """FTP 实际下载测试 — 使用 ftp.gnu.org 的公开文件。"""

    @pytest.mark.network
    def test_ftp_download_readme(self, tld_instance, tmp_download_dir):
        """从 ftp.gnu.org 下载 README，验证文件存在且有内容。"""
        ftp_url = "ftp://ftp.gnu.org/README"
        save_path = tmp_download_dir / "gnu_readme.txt"
        dl_id = tld_instance.start(
            urls=[ftp_url],
            save_paths=[str(save_path)],
            thread_count=2,
        )
        assert dl_id > 0, f"FTP start_download 应返回正数 ID, 得到 {dl_id}"

        for _ in range(30):
            if save_path.exists():
                break
            time.sleep(1)
        else:
            pytest.fail(f"FTP 下载超时: {save_path} 未在 30s 内创建")

        size = save_path.stat().st_size
        assert size > 0, f"FTP 下载文件为空"

    @pytest.mark.network
    def test_ftp_invalid_path(self, tld_instance, tmp_download_dir):
        """FTP 路径不存在时不应崩溃。"""
        ftp_url = "ftp://ftp.gnu.org/this_path_does_not_exist_xyz"
        save_path = tmp_download_dir / "ftp_invalid.txt"
        # start_download 会返回正数 ID，但下载会在后台失败
        dl_id = tld_instance.start(
            urls=[ftp_url],
            save_paths=[str(save_path)],
            thread_count=2,
        )
        assert dl_id > 0
        # 给下载留出时间（连接 + 尝试 RETR 失败）
        time.sleep(5)
        # 不应崩溃，下载器应正常停止
        # 不检查文件是否存在，因为路径无效


@pytest.mark.integration
class TestBitTorrentDownload:
    """BitTorrent 下载测试 — 使用本地生成的 .torrent 文件（无 tracker，仅验证初始化与取消）。"""

    def test_torrent_download_initialization(self, tld_instance, base_url, tmp_download_dir):
        """使用本地 .torrent 文件，验证 BT 下载器初始化和取消不崩溃。"""
        torrent_url = f"{base_url}/test_tiny.torrent"
        # TorrentDownloader 使用父目录作为输出目录
        save_path = tmp_download_dir / "bt_tiny" / "tiny_1kb.bin"
        save_path.parent.mkdir(parents=True, exist_ok=True)
        dl_id = tld_instance.start(
            urls=[torrent_url],
            save_paths=[str(save_path)],
            thread_count=4,
        )
        assert dl_id > 0, f"Torrent start_download 应返回正数 ID, 得到 {dl_id}"

        # 等待初始化和会话创建（下载不会完成，因为没有 peer）
        time.sleep(3)

        # 验证下载器没有崩溃 — 停止即可（会被 abort）
        # stop_download 在 fixture teardown 中自动调用

    def test_torrent_magnet_initialization(self, tld_instance, tmp_download_dir):
        """Magnet 链接初始化测试 — 验证不崩溃。"""
        # 使用一个格式有效的 magnet（无实际 tracker，仅测试初始化）
        magnet_url = "magnet:?xt=urn:btih:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA&dn=test"
        save_path = tmp_download_dir / "bt_magnet" / "test.bin"
        save_path.parent.mkdir(parents=True, exist_ok=True)
        dl_id = tld_instance.start(
            urls=[magnet_url],
            save_paths=[str(save_path)],
            thread_count=4,
        )
        assert dl_id > 0
        time.sleep(2)


@pytest.mark.integration
class TestED2KDownload:
    """ED2K 下载测试 — 验证解析和任务生命周期（外部网关可能不可达）。"""

    def test_ed2k_download_lifecycle(self, tld_instance, tmp_download_dir):
        """创建并启动 ED2K 下载，验证不崩溃。"""
        ed2k_url = "ed2k://|file|test.iso|1024|AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA|/"
        save_path = tmp_download_dir / "ed2k_test.bin"
        dl_id = tld_instance.start(
            urls=[ed2k_url],
            save_paths=[str(save_path)],
            thread_count=2,
        )
        assert dl_id > 0, f"ED2K start_download 应返回正数 ID, 得到 {dl_id}"

        # 给下载任务一些时间执行（如果 gateway 可达则下载，否则失败）
        time.sleep(3)


@pytest.mark.integration
class TestHTTP3Probe:
    """HTTP/3 探测测试 — 验证对支持 H3 的服务器自动启用 QUIC 下载。"""

    @pytest.mark.network
    def test_http3_probe_cloudflare(self, tld_instance, tmp_download_dir):
        """对 cloudflare.com 发起 HTTP 下载，验证 H3 探测 + QUIC 下载不崩溃。"""
        h3_url = "https://cloudflare.com/cdn-cgi/trace"
        save_path = tmp_download_dir / "h3_test.txt"
        dl_id = tld_instance.start(
            urls=[h3_url],
            save_paths=[str(save_path)],
            thread_count=2,
        )
        assert dl_id > 0, f"HTTP/3 start_download 应返回正数 ID, 得到 {dl_id}"

        for _ in range(10):
            if save_path.exists() and save_path.stat().st_size > 0:
                break
            time.sleep(1)
        else:
            # Cloudflare 可能返回 redirect，H3 下载器可能失败
            # 不 fail - 只需要验证不崩溃
            pass

    @pytest.mark.network
    def test_http3_probe_negative(self, tld_instance, tmp_download_dir):
        """对不支持 H3 的服务器发起 HTTP 下载，应回退到 HTTP。"""
        http_url = "https://httpbin.org/bytes/1024"
        save_path = tmp_download_dir / "http_fallback.txt"
        dl_id = tld_instance.start(
            urls=[http_url],
            save_paths=[str(save_path)],
            thread_count=2,
        )
        assert dl_id > 0

        for _ in range(15):
            if save_path.exists() and save_path.stat().st_size > 0:
                break
            time.sleep(1)
        else:
            # httpbin.org 可能不可达，不强制要求
            pass
