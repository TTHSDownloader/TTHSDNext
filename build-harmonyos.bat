@echo off
REM HarmonyOS 动态库构建脚本
REM 用法: build-harmonyos.bat [架构]
REM 架构选项: arm64-v8a (默认), armeabi-v7a, x86_64, x86

setlocal enabledelayedexpansion

REM 设置默认架构
set ARCH=%1
if "%ARCH%=="" set ARCH=arm64-v8a

echo ========================================
echo   TTHSD HarmonyOS 动态库构建脚本
echo ========================================
echo.

REM 检查 OHOS NDK 路径
set OHOS_NDK=C:\Users\sxxyrry_XR\AppData\Local\Huawei\Sdk\ohos-ndk
if not exist "%OHOS_NDK%" (
    echo [错误] 找不到 HarmonyOS NDK
    echo 请修改脚本中的 OHOS_NDK 为正确的路径
    echo 或设置环境变量 OHOS_NDK
    exit /b 1
)

echo [信息] 使用 HarmonyOS NDK: %OHOS_NDK%
echo [信息] 目标架构: %ARCH%
echo.

REM 映射架构名称到 Rust 目标
if "%ARCH%=="arm64-v8a" (
    set RUST_TARGET=aarch64-unknown-linux-ohos
    set OHOS_ARCH=arm64-v8a
) else if "%ARCH%=="armeabi-v7a" (
    set RUST_TARGET=armv7-unknown-linux-ohos
    set OHOS_ARCH=armeabi-v7a
) else if "%ARCH%=="x86" (
    set RUST_TARGET=i686-unknown-linux-ohos
    set OHOS_ARCH=x86
) else if "%ARCH%=="x86_64" (
    set RUST_TARGET=x86_64-unknown-linux-ohos
    set OHOS_ARCH=x86_64
) else (
    echo [错误] 不支持的架构: %ARCH%
    echo 支持的架构: arm64-v8a, armeabi-v7a, x86, x86_64
    exit /b 1
)

echo [信息] Rust 目标: %RUST_TARGET%
echo [信息] HarmonyOS 架构: %OHOS_ARCH%
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
set OUTPUT_DIR=HarmonyOS\libs\%OHOS_ARCH%
if not exist "%OUTPUT_DIR%" mkdir "%OUTPUT_DIR%"

echo [信息] 开始编译...
echo.

REM 编译项目（使用自定义配置文件）
cargo build --target %RUST_TARGET% --release --config .cargo/config-harmonyos.toml

if errorlevel 1 (
    echo [错误] 编译失败
    exit /b 1
)

echo.
echo [信息] 复制库文件到 %OUTPUT_DIR%...

REM 复制生成的 .so 文件
copy target\%RUST_TARGET%\release\tthsd.so "%OUTPUT_DIR%\tthsd_harmonyos.so" >nul

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
echo 1. 将 HarmonyOS/libs 文件夹复制到 HarmonyOS 项目的 entry/libs/ 目录
echo 2. 在 build-profile.json5 中配置 abiFilters: ["%OHOS_ARCH%"]
echo 3. 在 ArkTS 代码中加载库: System.loadLibrary("tthsd_harmonyos")
echo 4. 调用 C 接口函数进行下载操作
echo.

endlocal