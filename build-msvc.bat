@echo off
call "C:\VisualStudioCommunity\VC\Auxiliary\Build\vcvarsall.bat" x64
set BINDGEN_EXTRA_CLANG_ARGS=--target=x86_64-pc-windows-msvc
cargo build --release %*
