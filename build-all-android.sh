#!/bin/bash
# 一键编译所有 Android 架构的构建脚本 (Linux/macOS)

set -e

echo "========================================"
echo "  TTHSD Android 全架构构建脚本"
echo "========================================"
echo ""

# 检测操作系统并设置 NDK 路径
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    NDK_PATH="$HOME/Library/Android/sdk/ndk/26.1.10909125"
    CONFIG_FILE=".cargo/config-macos.toml"
else
    # Linux
    NDK_PATH="$HOME/Android/Sdk/ndk/26.1.10909125"
    CONFIG_FILE=".cargo/config-linux.toml"
fi

# 检查 NDK 路径是否存在
if [ ! -d "$NDK_PATH" ]; then
    echo "[错误] 找不到 Android NDK"
    echo "请确保已安装 Android NDK"
    echo "预期路径: $NDK_PATH"
    exit 1
fi

echo "[信息] 使用 NDK: $NDK_PATH"
echo "[信息] 使用配置: $CONFIG_FILE"
echo ""

# 定义要编译的架构
ARCHS=("arm64-v8a" "armeabi-v7a" "x86" "x86_64")

# 检查是否已安装所有 Rust 目标
echo "[信息] 检查 Rust 目标..."
for ARCH in "${ARCHS[@]}"; do
    case "$ARCH" in
        arm64-v8a)
            TARGET="aarch64-linux-android"
            ;;
        armeabi-v7a)
            TARGET="armv7-linux-androideabi"
            ;;
        x86)
            TARGET="i686-linux-android"
            ;;
        x86_64)
            TARGET="x86_64-linux-android"
            ;;
    esac

    if ! rustup target list --installed | grep -q "$TARGET"; then
        echo "  [安装] $TARGET"
        rustup target add "$TARGET"
    else
        echo "  [已安装] $TARGET"
    fi
done

echo ""
echo "[信息] 开始编译所有架构..."
echo ""

# 编译每个架构
SUCCESS_COUNT=0
FAILED_COUNT=0

for ARCH in "${ARCHS[@]}"; do
    echo "----------------------------------------"
    echo "[编译] $ARCH"
    echo "----------------------------------------"

    # 使用脚本编译单个架构
    if ./build-android.sh "$ARCH"; then
        echo "[成功] $ARCH 编译完成"
        ((SUCCESS_COUNT++))
    else
        echo "[失败] $ARCH 编译失败"
        ((FAILED_COUNT++))
    fi

    echo ""
done

# 显示编译结果摘要
echo "========================================"
echo "  编译完成"
echo "========================================"
echo "成功: $SUCCESS_COUNT"
echo "失败: $FAILED_COUNT"
echo ""

if [ $FAILED_COUNT -eq 0 ]; then
    echo "[信息] 所有架构编译成功!"
    echo ""
    echo "输出目录结构:"
    tree jniLibs 2>/dev/null || find jniLibs -type f
    echo ""
    echo "使用方法:"
    echo "将 jniLibs 文件夹复制到 Android 项目的 src/main/ 目录"
else
    echo "[警告] 部分架构编译失败，请检查上方错误信息"
    exit 1
fi