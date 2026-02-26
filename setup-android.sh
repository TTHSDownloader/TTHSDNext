#!/bin/bash
# Android 编译环境设置脚本 (Linux/macOS)
# 此脚本会自动配置 .cargo/config.toml

set -e

echo "========================================"
echo "  TTHSD Android 编译环境设置"
echo "========================================"
echo ""

# 检测操作系统
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "[信息] 检测到 macOS"
    NDK_PATH="$HOME/Library/Android/sdk/ndk"
    CONFIG_SRC=".cargo/config-macos.toml"
else
    echo "[信息] 检测到 Linux"
    NDK_PATH="$HOME/Android/Sdk/ndk"
    CONFIG_SRC=".cargo/config-linux.toml"
fi

echo "[信息] 查找 Android NDK..."
echo ""

# 查找最新的 NDK 版本
if [ -d "$NDK_PATH" ]; then
    # 获取最新的 NDK 版本目录
    LATEST_NDK=$(ls -t "$NDK_PATH" 2>/dev/null | head -n 1)

    if [ -n "$LATEST_NDK" ]; then
        echo "[信息] 找到 NDK: $NDK_PATH/$LATEST_NDK"
        echo ""

        # 更新配置文件中的 NDK 版本号
        if [ -f "$CONFIG_SRC" ]; then
            cp "$CONFIG_SRC" ".cargo/config.toml"
            sed -i.tmp "s|ndk/[0-9.]*|ndk/$LATEST_NDK|g" .cargo/config.toml
            rm -f .cargo/config.toml.tmp

            echo "[成功] 已配置 .cargo/config.toml"
            echo "  NDK 版本: $LATEST_NDK"
        else
            echo "[错误] 找不到配置文件: $CONFIG_SRC"
            exit 1
        fi
    else
        echo "[错误] NDK 目录存在但没有找到版本"
        exit 1
    fi
else
    echo "[错误] 找不到 Android NDK 目录"
    echo "请先安装 Android NDK"
    echo "预期路径: $NDK_PATH"
    exit 1
fi

echo ""
echo "[信息] 检查并安装 Rust 目标..."

# 检查并安装所有 Android 目标
TARGETS=(
    "aarch64-linux-android"
    "armv7-linux-androideabi"
    "i686-linux-android"
    "x86_64-linux-android"
)

for TARGET in "${TARGETS[@]}"; do
    if ! rustup target list --installed | grep -q "$TARGET"; then
        echo "  [安装] $TARGET"
        rustup target add "$TARGET"
    else
        echo "  [已安装] $TARGET"
    fi
done

echo ""
echo "========================================"
echo "  设置完成!"
echo "========================================"
echo ""
echo "现在可以运行以下命令编译 Android SO:"
echo "  ./build-android.sh arm64-v8a"
echo "  ./build-all-android.sh"
echo ""