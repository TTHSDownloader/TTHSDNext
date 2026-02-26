@echo off
REM 一键编译所有 Android 架构的构建脚本 (Windows)

setlocal enabledelayedexpansion

echo ========================================
echo   TTHSD Android 全架构构建脚本
echo ========================================
echo.

REM 检查 NDK 路径
set NDK_PATH=C:\Users\sxxyrry_XR\AppData\Local\Android\Sdk\ndk\26.1.10909125
if not exist "%NDK_PATH%" (
    echo [错误] 找不到 Android NDK
    echo 请确保已安装 Android NDK
    echo 预期路径: %NDK_PATH%
    exit /b 1
)

echo [信息] 使用 NDK: %NDK_PATH%
echo.

REM 检查并安装所有 Rust 目标
echo [信息] 检查 Rust 目标...

rustup target list --installed | findstr /C:"aarch64-linux-android" >nul
if errorlevel 1 (
    echo   [安装] aarch64-linux-android
    rustup target add aarch64-linux-android
) else (
    echo   [已安装] aarch64-linux-android
)

rustup target list --installed | findstr /C:"armv7-linux-androideabi" >nul
if errorlevel 1 (
    echo   [安装] armv7-linux-androideabi
    rustup target add armv7-linux-androideabi
) else (
    echo   [已安装] armv7-linux-androideabi
)

rustup target list --installed | findstr /C:"i686-linux-android" >nul
if errorlevel 1 (
    echo   [安装] i686-linux-android
    rustup target add i686-linux-android
) else (
    echo   [已安装] i686-linux-android
)

rustup target list --installed | findstr /C:"x86_64-linux-android" >nul
if errorlevel 1 (
    echo   [安装] x86_64-linux-android
    rustup target add x86_64-linux-android
) else (
    echo   [已安装] x86_64-linux-android
)

echo.
echo [信息] 开始编译所有架构...
echo.

set SUCCESS_COUNT=0
set FAILED_COUNT=0

REM 编译 arm64-v8a
echo ----------------------------------------
echo [编译] arm64-v8a
echo ----------------------------------------
call build-android.bat arm64-v8a
if errorlevel 1 (
    echo [失败] arm64-v8a 编译失败
    set /a FAILED_COUNT+=1
) else (
    echo [成功] arm64-v8a 编译完成
    set /a SUCCESS_COUNT+=1
)
echo.

REM 编译 armeabi-v7a
echo ----------------------------------------
echo [编译] armeabi-v7a
echo ----------------------------------------
call build-android.bat armeabi-v7a
if errorlevel 1 (
    echo [失败] armeabi-v7a 编译失败
    set /a FAILED_COUNT+=1
) else (
    echo [成功] armeabi-v7a 编译完成
    set /a SUCCESS_COUNT+=1
)
echo.

REM 编译 x86
echo ----------------------------------------
echo [编译] x86
echo ----------------------------------------
call build-android.bat x86
if errorlevel 1 (
    echo [失败] x86 编译失败
    set /a FAILED_COUNT+=1
) else (
    echo [成功] x86 编译完成
    set /a SUCCESS_COUNT+=1
)
echo.

REM 编译 x86_64
echo ----------------------------------------
echo [编译] x86_64
echo ----------------------------------------
call build-android.bat x86_64
if errorlevel 1 (
    echo [失败] x86_64 编译失败
    set /a FAILED_COUNT+=1
) else (
    echo [成功] x86_64 编译完成
    set /a SUCCESS_COUNT+=1
)
echo.

REM 显示编译结果摘要
echo ========================================
echo   编译完成
echo ========================================
echo 成功: %SUCCESS_COUNT%
echo 失败: %FAILED_COUNT%
echo.

if %FAILED_COUNT%==0 (
    echo [信息] 所有架构编译成功!
    echo.
    echo 输出目录结构:
    dir /s /b jniLibs\*.so
    echo.
    echo 使用方法:
    echo 将 jniLibs 文件夹复制到 Android 项目的 src/main/ 目录
) else (
    echo [警告] 部分架构编译失败，请检查上方错误信息
    exit /b 1
)

endlocal