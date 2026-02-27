#!/bin/bash
# HarmonyOS 动态库构建脚本
# 用法: ./build-harmonyos.sh [架构]
# 架构选项: arm64-v8a (默认), armeabi-v7a, x86_64, x86

# 设置默认架构
ARCH=${1:-arm64-v8a}

echo "========================================"
echo "  TTHSD HarmonyOS 动态库构建脚本"
echo "========================================"
echo ""

# 检查 OHOS NDK 路径
OHOS_NDK=${OHOS_NDK:-"$HOME/Library/Huawei/Sdk/ohos-ndk"}
if [ ! -d "$OHOS_NDK" ]; then
    echo "[错误] 找不到 HarmonyOS NDK"
    echo "请设置环境变量 OHOS_NDK 为正确的路径"
    exit 1
fi

echo "[信息] 使用 HarmonyOS NDK: $OHOS_NDK"
echo "[信息] 目标架构: $ARCH"
echo ""

# 映射架构名称到 Rust 目标
case $ARCH in
    arm64-v8a)
        RUST_TARGET=aarch64-unknown-linux-ohos
        OHOS_ARCH=arm64-v8a
        ;;
    armeabi-v7a)
        RUST_TARGET=armv7-unknown-linux-ohos
        OHOS_ARCH=armeabi-v7a
        ;;
    x86)
        RUST_TARGET=i686-unknown-linux-ohos
        OHOS_ARCH=x86
        ;;
    x86_64)
        RUST_TARGET=x86_64-unknown-linux-ohos
        OHOS_ARCH=x86_64
        ;;
    *)
        echo "[错误] 不支持的架构: $ARCH"
        echo "支持的架构: arm64-v8a, armeabi-v7a, x86, x86_64"
        exit 1
        ;;
esac

echo "[信息] Rust 目标: $RUST_TARGET"
echo "[信息] HarmonyOS 架构: $OHOS_ARCH"
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
OUTPUT_DIR="HarmonyOS/libs/$OHOS_ARCH"
mkdir -p "$OUTPUT_DIR"

echo "[信息] 开始编译..."
echo ""

# 编译项目（使用自定义配置文件）
cargo build --target "$RUST_TARGET" --release --config .cargo/config-harmonyos.toml

if [ $? -ne 0 ]; then
    echo "[错误] 编译失败"
    exit 1
fi

echo ""
echo "[信息] 复制库文件到 $OUTPUT_DIR..."

# 复制生成的 .so 文件
cp "target/$RUST_TARGET/release/tthsd.so" "$OUTPUT_DIR/tthsd_harmonyos.so"

if [ $? -ne 0 ]; then
    echo "[错误] 复制文件失败"
    exit 1
fi

echo ""
echo "========================================"
echo "  构建成功!"
echo "========================================"
echo "输出文件: $OUTPUT_DIR/tthsd_harmonyos.so"
echo ""
echo "使用方法:"
echo "1. 将 HarmonyOS/libs 文件夹复制到 HarmonyOS 项目的 entry/libs/ 目录"
echo "2. 在 build-profile.json5 中配置 abiFilters: [\"$OHOS_ARCH\"]"
echo "3. 在 ArkTS 代码中加载库: System.loadLibrary(\"tthsd_harmonyos\")"
echo "4. 调用 C 接口函数进行下载操作"
echo ""