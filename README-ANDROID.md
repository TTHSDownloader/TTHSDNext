# TTHSD Android 编译指南

## 编译为 Android 动态库 (.so)

### 前置要求

1. **Rust 工具链**
   ```bash
   rustup --version
   ```

2. **Android NDK**
   - 已配置路径: `C:\Users\sxxyrry_XR\AppData\Local\Android\Sdk\ndk\26.1.10909125`
   - 如需修改，请编辑 `.cargo/config.toml`

3. **添加 Android 编译目标**
   ```bash
   rustup target add aarch64-linux-android
   rustup target add armv7-linux-androideabi
   rustup target add i686-linux-android
   rustup target add x86_64-linux-android
   ```

### 编译方法

#### 方法一：使用构建脚本（推荐）

```bash
# 编译 arm64-v8a（64位 ARM，最常用）
build-android.bat

# 编译 armeabi-v7a（32位 ARM）
build-android.bat armeabi-v7a

# 编译 x86（32位 x86 模拟器）
build-android.bat x86

# 编译 x86_64（64位 x86 模拟器）
build-android.bat x86_64
```

输出文件位于 `jniLibs\{架构}\libTTHSD.so`

#### 方法二：使用 cargo-ndk（更灵活）

```bash
# 安装 cargo-ndk
cargo install cargo-ndk

# 编译所有架构
cargo ndk -t arm64-v7a -t arm64-v8a -t x86 -t x86_64 -o ./jniLibs build --release --features android
```

#### 方法三：直接使用 cargo

```bash
# 编译 arm64-v8a
cargo build --target aarch64-linux-android --release --features android

# 输出文件: target\aarch64-linux-android\release\libTTHSD.so
```

### 在 Android 项目中使用

1. **复制库文件**
   ```
   将 jniLibs 文件夹复制到 Android 项目的 src/main/ 目录下
   ```

2. **Kotlin 代码示例**
   ```kotlin
   package com.tthsd

   class TTHSDLibrary {
       init {
           System.loadLibrary("TTHSD")
       }

       // 声明原生方法
       external fun startDownload(
           tasksJson: String,
           threadCount: Int,
           chunkSizeMb: Int,
           useCallbackUrl: Boolean,
           callbackUrl: String,
           useSocket: Boolean,
           isMultiple: Boolean
       ): Int

       external fun pauseDownload(id: Int): Int

       external fun resumeDownload(id: Int): Int

       external fun stopDownload(id: Int): Int
   }
   ```

3. **在 app/build.gradle 中配置**
   ```gradle
   android {
       // ...
       sourceSets {
           main {
               jniLibs.srcDirs = ['src/main/jniLibs']
           }
       }
   }
   ```

### 支持的架构

| 架构名称 | Rust 目标 | 说明 |
|---------|-----------|------|
| arm64-v8a | aarch64-linux-android | 64位 ARM（推荐） |
| armeabi-v7a | armv7-linux-androideabi | 32位 ARM |
| x86 | i686-linux-android | 32位 x86（模拟器） |
| x86_64 | x86_64-linux-android | 64位 x86（模拟器） |

### 编译 Windows DLL（保留原有功能）

```bash
# 编译 Windows 动态库
cargo build --release

# 输出文件: target\release\TTHSD.dll
```

### 编译其他平台

```bash
# Linux 动态库 (.so)
cargo build --target x86_64-unknown-linux-gnu --release

# macOS 动态库 (.dylib)
cargo build --target x86_64-apple-darwin --release
```

### 常见问题

**Q: 编译时找不到 NDK？**
A: 检查 `.cargo/config.toml` 中的 NDK 路径是否正确。

**Q: 编译失败，提示链接器错误？**
A: 确保已安装对应架构的 Rust 目标，并检查 NDK 版本是否兼容。

**Q: 如何在 Android 中调用 Rust 函数？**
A: 使用 JNI，示例代码在 `src/core/android_export.rs` 中。

**Q: Android 版本和 Windows 版本有什么区别？**
A: Android 版本通过 JNI 与 Java/Kotlin 交互，Windows 版本直接导出 C 函数。核心功能完全相同。