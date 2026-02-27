@echo off
REM 编译所有 HarmonyOS 架构的动态库

echo ========================================
echo   编译所有 HarmonyOS 架构
echo ========================================
echo.

call build-harmonyos.bat arm64-v8a
call build-harmonyos.bat armeabi-v7a
call build-harmonyos.bat x86_64
call build-harmonyos.bat x86

echo.
echo ========================================
echo   所有架构编译完成!
echo ========================================
echo 输出目录: HarmonyOS/libs/
echo.