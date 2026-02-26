#!/bin/bash
# 编译所有 HarmonyOS 架构的动态库

echo "========================================"
echo "  编译所有 HarmonyOS 架构"
echo "========================================"
echo ""

./build-harmonyos.sh arm64-v8a
./build-harmonyos.sh armeabi-v7a
./build-harmonyos.sh x86_64
./build-harmonyos.sh x86

echo ""
echo "========================================"
echo "  所有架构编译完成!"
echo "========================================"
echo "输出目录: HarmonyOS/libs/"
echo ""