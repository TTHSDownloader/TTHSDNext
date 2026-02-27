# TTHSD HarmonyOS 编译指南

## 简介

本指南介绍如何将 TTHSD 高速下载器编译为 HarmonyOS 平台的动态库 (.so)，并在 HarmonyOS 应用中使用。

> **注意**: HarmonyOS 支持 Linux 编译的 .so 文件，但需要使用 HarmonyOS NDK 进行交叉编译以确保兼容性。本项目的现有 C 接口代码可以直接用于 HarmonyOS 平台，无需修改。

## 前置要求

### 1. Rust 工具链

```bash
rustc --version
cargo --version
```

### 2. HarmonyOS NDK

- 下载地址: [HarmonyOS NDK](https://developer.huawei.com/consumer/cn/doc/harmonyos-guides-V5/ide-command-line-0000001774121590-V5)
- 推荐版本: API 12 或更高版本

**Windows 默认路径**:
```
C:\Users\{用户名}\AppData\Local\Huawei\Sdk\ohos-ndk
```

**macOS/Linux 默认路径**:
```
~/Library/Huawei/Sdk/ohos-ndk  # macOS
~/Huawei/Sdk/ohos-ndk          # Linux
```

如需修改路径，请编辑 `.cargo/config-harmonyos.toml` 文件中的 `OHOS_NDK` 环境变量。

### 3. 添加 Rust 编译目标

```bash
# 64位 ARM (推荐，用于真实设备)
rustup target add aarch64-unknown-linux-ohos

# 32位 ARM (用于旧设备)
rustup target add armv7-unknown-linux-ohos

# 64位 x86 (用于模拟器)
rustup target add x86_64-unknown-linux-ohos

# 32位 x86 (用于模拟器)
rustup target add i686-unknown-linux-ohos
```

## 编译方法

### 方法一：使用构建脚本（推荐）

#### Windows

```bash
# 编译 arm64-v8a（64位 ARM，最常用）
build-harmonyos.bat

# 编译 armeabi-v7a（32位 ARM）
build-harmonyos.bat armeabi-v7a

# 编译 x86（32位 x86 模拟器）
build-harmonyos.bat x86

# 编译 x86_64（64位 x86 模拟器）
build-harmonyos.bat x86_64

# 编译所有架构
build-all-harmonyos.bat
```

#### macOS/Linux

```bash
# 添加执行权限
chmod +x build-harmonyos.sh build-all-harmonyos.sh

# 编译 arm64-v8a（64位 ARM，最常用）
./build-harmonyos.sh

# 编译 armeabi-v7a（32位 ARM）
./build-harmonyos.sh armeabi-v7a

# 编译 x86（32位 x86 模拟器）
./build-harmonyos.sh x86

# 编译 x86_64（64位 x86 模拟器）
./build-harmonyos.sh x86_64

# 编译所有架构
./build-all-harmonyos.sh
```

**输出文件位置**: `HarmonyOS/libs/{架构}/libTTHSD.so`

### 方法二：直接使用 cargo

```bash
# 编译 arm64-v8a
cargo build --target aarch64-unknown-linux-ohos --release --config .cargo/config-harmonyos.toml

# 输出文件: target/aarch64-unknown-linux-ohos/release/libTTHSD.so
```

## 在 HarmonyOS 项目中使用

### 1. 配置项目结构

将编译好的库文件复制到 HarmonyOS 项目的对应目录：

```
YourHarmonyOSProject/
├── entry/
│   ├── libs/
│   │   ├── arm64-v8a/
│   │   │   └── libTTHSD.so
│   │   ├── armeabi-v7a/
│   │   │   └── libTTHSD.so
│   │   ├── x86/
│   │   │   └── libTTHSD.so
│   │   └── x86_64/
│   │       └── libTTHSD.so
```

### 2. 配置 build-profile.json5

在 `entry/build-profile.json5` 中添加 abiFilters 配置：

```json5
{
  "app": {
    "buildOption": {
      "externalNativeOptions": {
        "path": "./src/main/cpp/CMakeLists.txt",
        "arguments": "",
        "cppFlags": "",
        "abiFilters": [
          "arm64-v8a",
          "armeabi-v7a",
          "x86",
          "x86_64"
        ]
      }
    }
  }
}
```

### 3. 配置 module.json5

在 `entry/src/main/module.json5` 中声明使用原生库：

```json5
{
  "module": {
    "requestPermissions": [
      {
        "name": "ohos.permission.INTERNET",
        "reason": "$string:internet_permission_reason",
        "usedScene": {
          "abilities": [
            "EntryAbility"
          ],
          "when": "always"
        }
      },
      {
        "name": "ohos.permission.GET_NETWORK_INFO",
        "reason": "$string:network_info_permission_reason",
        "usedScene": {
          "abilities": [
            "EntryAbility"
          ],
          "when": "always"
        }
      }
    ]
  }
}
```

### 4. ArkTS 代码示例

在 `entry/src/main/ets/pages/Index.ets` 中使用：

```typescript
import { nativeModule } from '@kit.ArkTS';

// 定义 C 接口函数
interface TTHSDLib {
  start_download(
    tasks_data: string,
    task_count: number,
    thread_count: number,
    chunk_size_mb: number,
    callback: number,
    use_callback_url: boolean,
    user_agent: string,
    remote_callback_url: string,
    use_socket: boolean,
    is_multiple: boolean
  ): number;

  pause_download(id: number): number;
  resume_download(id: number): number;
  stop_download(id: number): number;
}

// 加载库
const tthsdLib: TTHSDLib = nativeModule.loadLibrary('TTHSD');

// 示例：启动下载
function startDownloadExample() {
  const tasks = [
    {
      "URL": "https://example.com/file.zip",
      "SavePath": "/data/storage/el2/base/haps/entry/files/file.zip",
      "ShowName": "示例文件.zip",
      "ID": "123e4567-e89b-12d3-a456-426614174000"
    }
  ];

  const tasksJson = JSON.stringify(tasks);
  const taskId = tthsdLib.start_download(
    tasksJson,
    1,
    4,
    10,
    0,
    false,
    "",
    "",
    false,
    false
  );

  console.log(`下载任务 ID: ${taskId}`);
}

// 示例：暂停下载
function pauseDownloadExample(taskId: number) {
  const result = tthsdLib.pause_download(taskId);
  console.log(`暂停下载结果: ${result === 0 ? '成功' : '失败'}`);
}

// 示例：恢复下载
function resumeDownloadExample(taskId: number) {
  const result = tthsdLib.resume_download(taskId);
  console.log(`恢复下载结果: ${result === 0 ? '成功' : '失败'}`);
}

// 示例：停止下载
function stopDownloadExample(taskId: number) {
  const result = tthsdLib.stop_download(taskId);
  console.log(`停止下载结果: ${result === 0 ? '成功' : '失败'}`);
}
```

## 支持的架构

| 架构名称 | Rust 目标 | 说明 |
|---------|-----------|------|
| arm64-v8a | aarch64-unknown-linux-ohos | 64位 ARM（推荐，用于真实设备） |
| armeabi-v7a | armv7-unknown-linux-ohos | 32位 ARM（用于旧设备） |
| x86_64 | x86_64-unknown-linux-ohos | 64位 x86（用于模拟器） |
| x86 | i686-unknown-linux-ohos | 32位 x86（用于模拟器） |

## C 接口函数说明

本项目的 C 接口函数在 HarmonyOS 上可以直接使用，无需修改。以下是主要的导出函数：

### start_download

启动下载任务

```c
int start_download(
    const char* tasks_data,      // JSON 格式的任务数据
    int task_count,              // 任务数量
    int thread_count,            // 下载线程数
    int chunk_size_mb,           // 分块大小 (MB)
    size_t callback,             // 回调函数指针（HarmonyOS 上通常传 0）
    bool use_callback_url,       // 是否使用回调 URL
    const char* user_agent,      // User-Agent 字符串
    const char* remote_callback_url, // 回调 URL 地址
    const bool* use_socket,      // 是否使用 Socket
    const bool* is_multiple      // 是否为多任务下载
);
```

**返回值**: 下载器 ID，失败返回 -1

### pause_download

暂停下载

```c
int pause_download(int id);  // 下载器 ID
```

**返回值**: 0 成功，-1 失败

### resume_download

恢复下载

```c
int resume_download(int id);  // 下载器 ID
```

**返回值**: 0 成功，-1 失败

### stop_download

停止下载

```c
int stop_download(int id);  // 下载器 ID
```

**返回值**: 0 成功，-1 失败

## 任务数据格式

任务数据使用 JSON 格式：

```json
[
  {
    "URL": "https://example.com/file1.zip",
    "SavePath": "/data/storage/el2/base/haps/entry/files/file1.zip",
    "ShowName": "文件1.zip",
    "ID": "3686b666-5716-477c-a364-5b4b4e684874"
  },
  {
    "URL": "https://example.com/file2.zip",
    "SavePath": "/data/storage/el2/base/haps/entry/files/file2.zip",
    "ShowName": "文件2.zip",
    "ID": "ac824bcc-ed02-4c4d-8b14-bfc500f0ba86"
  }
]
```

**注意事项**:
- `SavePath` 必须是 HarmonyOS 应用的可写路径
- 推荐使用 `/data/storage/el2/base/haps/entry/files/` 作为基础路径
- `ID` 必须是唯一的字符串

## 常见问题

### Q: 编译时找不到 HarmonyOS NDK？

**A**: 检查 `.cargo/config-harmonyos.toml` 中的 OHOS_NDK 路径是否正确，或设置环境变量 `OHOS_NDK`。

### Q: 编译失败，提示链接器错误？

**A**:
1. 确保已安装对应架构的 Rust 目标
2. 检查 HarmonyOS NDK 版本是否兼容
3. 确保 NDK 中的 LLVM 工具链完整

### Q: 如何在 HarmonyOS 中获取应用的存储路径？

**A**: 使用以下 ArkTS 代码获取：

```typescript
import { Context } from '@kit.AbilityKit';

const context = getContext(this) as Context;
const filesDir = context.filesDir;
// 输出: /data/storage/el2/base/haps/entry/files
```

### Q: HarmonyOS 版本和 Android 版本有什么区别？

**A**:
- HarmonyOS 版本使用 `ohos-unknown-linux-ohos` target
- Android 版本使用 `*-linux-android` target
- 两者都使用相同的 C 接口函数
- HarmonyOS 版本在 HarmonyOS 应用中通过 ArkTS 调用
- Android 版本在 Android 应用中通过 Java/Kotlin 调用

### Q: 如何调试 HarmonyOS 原生库？

**A**:
1. 使用 `hdc` 命令查看日志: `hdc shell hilog | grep TTHSD`
2. 在代码中添加 `eprintln!` 宏输出调试信息
3. 使用 HarmonyOS Studio 的原生调试功能

## 与 Android 的兼容性

由于 HarmonyOS 和 Android 都基于 Linux 内核，本项目的核心下载逻辑在两个平台上是通用的。主要区别在于：

1. **编译目标**: `*-linux-android` vs `*-unknown-linux-ohos`
2. **NDK 工具链**: Android NDK vs HarmonyOS NDK
3. **调用方式**: Java/Kotlin JNI vs ArkTS Native API
4. **权限系统**: Android 权限 vs HarmonyOS 权限

**重要**: HarmonyOS 不直接支持 Java/Kotlin 的 JNI 接口，因此 Android 版本的 `android_export.rs` 中的 JNI 函数不能在 HarmonyOS 上使用。请使用 C 接口函数（`export.rs` 中定义的函数）。

## 更多信息

- [HarmonyOS NDK 文档](https://developer.huawei.com/consumer/cn/doc/harmonyos-guides-V5/ide-command-line-0000001774121590-V5)
- [TTHSD 主文档](../README.md)
- [API 说明](https://docss.sxxyrry.qzz.io/TTHSD/zh/api/API-overview.html)

## 许可证

本项目基于 [GNU General Public License v3.0](https://www.gnu.org/licenses/gpl-3.0.en.html) 开源发布。
