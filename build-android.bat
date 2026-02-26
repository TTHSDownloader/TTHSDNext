@echo off
REM Android 动态库构建脚本
REM 用法: build-android.bat [架构]
REM 架构选项: arm64-v8a (默认), armeabi-v7a, x86, x86_64

setlocal enabledelayedexpansion

REM 设置默认架构
set ARCH=%1
if "%ARCH%=="" set ARCH=arm64-v8a

echo ========================================
echo   TTHSD Android 动态库构建脚本
echo ========================================
echo.

REM 检查 NDK 路径
set NDK_PATH=C:\Users\sxxyrry_XR\AppData\Local\Android\Sdk\ndk\26.1.10909125
if not exist "%NDK_PATH%" (
    echo [错误] 找不到 Android NDK
    echo 请修改脚本中的 NDK_PATH 为正确的路径
    exit /b 1
)

echo [信息] 使用 NDK: %NDK_PATH%
echo [信息] 目标架构: %ARCH%
echo.

REM 映射架构名称到 Rust 目标
if "%ARCH%=="arm64-v8a" (
    set RUST_TARGET=aarch64-linux-android
) else if "%ARCH%=="armeabi-v7a" (
    set RUST_TARGET=armv7-linux-androideabi
) else if "%ARCH%=="x86" (
    set RUST_TARGET=i686-linux-android
) else if "%ARCH%=="x86_64" (
    set RUST_TARGET=x86_64-linux-android
) else (
    echo [错误] 不支持的架构: %ARCH%
    echo 支持的架构: arm64-v8a, armeabi-v7a, x86, x86_64
    exit /b 1
)

echo [信息] Rust 目标: %RUST_TARGET%
echo.

REM 检查是否已添加 Rust 目标
rustup target list | findstr /C:"%RUST_TARGET% (installed)" >nul
if errorlevel 1 (
    echo [警告] 未安装 Rust 目标 %RUST_TARGET%
    echo 正在安装...
    rustup target add %RUST_TARGET%
    if errorlevel 1 (
        echo [错误] 安装 Rust 目标失败
        exit /b 1
    )
    echo [信息] Rust 目标安装成功
    echo.
)

REM 创建输出目录
set OUTPUT_DIR=jniLibs\%ARCH%
if not exist "%OUTPUT_DIR%" mkdir "%OUTPUT_DIR%"

echo [信息] 开始编译...
echo.

REM 编译项目
cargo build --target %RUST_TARGET% --release --features android

if errorlevel 1 (
    echo [错误] 编译失败
    exit /b 1
)

echo.
echo [信息] 复制库文件到 %OUTPUT_DIR%...

REM 复制生成的 .so 文件
copy target\%RUST_TARGET%\release\libTTHSD.so "%OUTPUT_DIR%\libTTHSD.so" >nul

if errorlevel 1 (
    echo [错误] 复制文件失败
    exit /b 1
)

echo.
echo ========================================
echo   构建成功!
echo ========================================
echo 输出文件: %OUTPUT_DIR%\libTTHSD.so
echo.
echo 使用方法:
echo 1. 将 jniLibs 文件夹复制到 Android 项目的 src/main/ 目录
echo 2. 在 Java/Kotlin 代码中加载库: System.loadLibrary("TTHSD")
echo.

endlocal