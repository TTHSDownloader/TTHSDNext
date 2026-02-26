# TTHSD Android Build Guide

This project supports building Android dynamic libraries (.so) on Windows, Linux, and macOS.

## Build Android Dynamic Library (.so)

### Prerequisites

1. **Rust Toolchain**
   ```bash
   rustup --version
   ```

2. **Android NDK**
   - Windows: `C:\Users\YourName\AppData\Local\Android\Sdk\ndk\26.1.10909125`
   - Linux: `~/Android/Sdk/ndk/26.1.10909125`
   - macOS: `~/Library/Android/sdk/ndk/26.1.10909125`
   - Modify paths in `.cargo/config.toml` if needed

3. **Add Android Build Targets**
   ```bash
   rustup target add aarch64-linux-android
   rustup target add armv7-linux-androideabi
   rustup target add i686-linux-android
   rustup target add x86_64-linux-android
   ```

### Build Methods

#### Windows System

**Using Build Script (Recommended)**

```bash
# Build arm64-v8a (64-bit ARM, most common)
build-android.bat

# Build armeabi-v7a (32-bit ARM)
build-android.bat armeabi-v7a

# Build x86 (32-bit x86 emulator)
build-android.bat x86

# Build x86_64 (64-bit x86 emulator)
build-android.bat x86_64
```

**Build All Architectures at Once**

```bash
build-all-android.bat
```

Output files are in `jniLibs\{arch}\libTTHSD.so`

#### Linux / macOS System

**First Time: Run Setup Script**

```bash
chmod +x setup-android.sh
./setup-android.sh
```

This script will automatically:
- Detect system type (Linux/macOS)
- Find and configure Android NDK path
- Install all Rust Android build targets

**Using Build Script (Recommended)**

```bash
# Add execute permission to scripts
chmod +x build-android.sh
chmod +x build-all-android.sh

# Build arm64-v8a (64-bit ARM, most common)
./build-android.sh arm64-v8a

# Build armeabi-v7a (32-bit ARM)
./build-android.sh armeabi-v7a

# Build x86 (32-bit x86 emulator)
./build-android.sh x86

# Build x86_64 (64-bit x86 emulator)
./build-android.sh x86_64
```

**Build All Architectures at Once**

```bash
./build-all-android.sh
```

Output files are in `jniLibs/{arch}/libTTHSD.so`

#### Method 2: Using cargo-ndk (Cross-platform, More Flexible)

```bash
# Install cargo-ndk
cargo install cargo-ndk

# Build all architectures
cargo ndk -t armeabi-v7a -t arm64-v8a -t x86 -t x86_64 -o ./jniLibs build --release --features android
```

#### Method 3: Manual .cargo/config.toml Configuration

If auto-detection fails, manually configure:

**Windows System**
- Copy `.cargo/config.toml.example` to `.cargo/config.toml`
- Modify NDK path to your actual path

**Linux System**
```bash
cp .cargo/config-linux.toml .cargo/config.toml
# Edit the file to modify NDK path
```

**macOS System**
```bash
cp .cargo/config-macos.toml .cargo/config.toml
# Edit the file to modify NDK path
```

#### Method 4: Direct cargo Command

```bash
# Build arm64-v8a
cargo build --target aarch64-linux-android --release --features android

# Output file: target\aarch64-linux-android\release\libTTHSD.so
```

### Using in Android Project

1. **Copy Library Files**
   ```
   Copy the jniLibs folder to src/main/ directory in your Android project
   ```

2. **Kotlin Code Example**
   ```kotlin
   package com.tthsd

   class TTHSDLibrary {
       init {
           System.loadLibrary("TTHSD")
       }

       // Declare native methods
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

3. **Configure in app/build.gradle**
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

### Supported Architectures

| Architecture | Rust Target | Description |
|--------------|-------------|-------------|
| arm64-v8a | aarch64-linux-android | 64-bit ARM (Recommended) |
| armeabi-v7a | armv7-linux-androideabi | 32-bit ARM |
| x86 | i686-linux-android | 32-bit x86 (Emulator) |
| x86_64 | x86_64-linux-android | 64-bit x86 (Emulator) |

### Build Windows DLL (Retain Original Functionality)

```bash
# Build Windows dynamic library
cargo build --release

# Output file: target\release\TTHSD.dll
```

### Build Other Platforms

```bash
# Linux dynamic library (.so)
cargo build --target x86_64-unknown-linux-gnu --release

# macOS dynamic library (.dylib)
cargo build --target x86_64-apple-darwin --release
```

### Common Questions

**Q: NDK not found during build?**
A:
- Windows: Check NDK path in `.cargo/config.toml`
- Linux/macOS: Run `./setup-android.sh` or check `.cargo/config.toml`

**Q: Build failed with linker error?**
A: Ensure Rust target for the architecture is installed and NDK version is compatible.

**Q: Cannot find clang on Linux/macOS?**
A: Run `./setup-android.sh` for auto-configuration, or manually edit NDK path in `.cargo/config.toml`.

**Q: How to call Rust functions in Android?**
A: Use JNI. Example code is in `src/core/android_export.rs`.

**Q: What's the difference between Android and Windows versions?**
A: Android version interacts with Java/Kotlin via JNI, Windows version exports C functions directly. Core functionality is identical.

**Q: Can I build on Windows and use the SO on Linux?**
A: No, you must build on each platform separately. However, generated SO files work on all Android devices regardless of build platform.