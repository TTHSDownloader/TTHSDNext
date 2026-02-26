#!/bin/bash
# Android 动态库构建脚本 (Linux/macOS)
# 用法: ./build-android.sh [架构]
# 架构选项: arm64-v8a (默认), armeabi-v7a, x86, x86_64

set -e

# 设置默认架构
ARCH=${1:-arm64-v8a}

echo "========================================"
echo "  TTHSD Android 动态库构建脚本"
echo "========================================"
echo ""

# 检测操作系统并设置 NDK 路径
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    NDK_PATH="$HOME/Library/Android/sdk/ndk/26.1.10909125"
else
    # Linux
    NDK_PATH="$HOME/Android/Sdk/ndk/26.1.10909125"
fi

# 检查 NDK 路径是否存在
if [ ! -d "$NDK_PATH" ]; then
    echo "[错误] 找不到 Android NDK"
    echo "请修改脚本中的 NDK_PATH 为正确的路径"
    echo "当前路径: $NDK_PATH"
    exit 1
fi

echo "[信息] 使用 NDK: $NDK_PATH"
echo "[信息] 目标架构: $ARCH"
echo ""

# 映射架构名称到 Rust 目标
case "$ARCH" in
    arm64-v8a)
        RUST_TARGET=aarch64-linux-android
        ;;
    armeabi-v7a)
        RUST_TARGET=armv7-linux-androideabi
        ;;
    x86)
        RUST_TARGET=i686-linux-android
        ;;
    x86_64)
        RUST_TARGET=x86_64-linux-android
        ;;
    *)
        echo "[错误] 不支持的架构: $ARCH"
        echo "支持的架构: arm64-v8a, armeabi-v7a, x86, x86_64"
        exit 1
        ;;
esac

echo "[信息] Rust 目标: $RUST_TARGET"
echo ""

# 检查是否已添加 Rust 目标
if ! rustup target list --installed | grep -q "$RUST_TARGET"; then
    echo "[警告] 未安装 Rust 目标 $RUST_TARGET"
    echo "正在安装..."
    rustup target add "$RUST_TARGET"
    if [ $? -ne 0 ]; then
        echo "[错误] 安装 Rust 目标失败"
        exit 1
    fi
    echo "[信息] Rust 目标安装成功"
    echo ""
fi

# 创建输出目录
OUTPUT_DIR="jniLibs/$ARCH"
mkdir -p "$OUTPUT_DIR"

echo "[信息] 开始编译..."
echo ""

# 编译项目
cargo build --target "$RUST_TARGET" --release --features android

if [ $? -ne 0 ]; then
    echo "[错误] 编译失败"
    exit 1
fi

echo ""
echo "[信息] 复制库文件到 $OUTPUT_DIR..."

# 复制生成的 .so 文件
cp "target/$RUST_TARGET/release/libTTHSD.so" "$OUTPUT_DIR/libTTHSD.so"

if [ $? -ne 0 ]; then
    echo "[错误] 复制文件失败"
    exit 1
fi

echo ""
echo "========================================"
echo "  构建成功!"
echo "========================================"
echo "输出文件: $OUTPUT_DIR/libTTHSD.so"
echo ""
echo "使用方法:"
echo "1. 将 jniLibs 文件夹复制到 Android 项目的 src/main/ 目录"
echo "2. 在 Java/Kotlin 代码中加载库: System.loadLibrary(\"TTHSD\")"
echo ""
