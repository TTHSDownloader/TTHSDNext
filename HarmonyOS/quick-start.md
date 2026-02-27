# HarmonyOS 快速开始

## 快速编译

### Windows

```batch
# 编译所有架构
build-all-harmonyos.bat

# 或只编译 arm64-v8a（推荐）
build-harmonyos.bat
```

### macOS/Linux

```bash
# 编译所有架构
./build-all-harmonyos.sh

# 或只编译 arm64-v8a（推荐）
./build-harmonyos.sh
```

## 快速集成

1. **复制库文件**
   ```
   将 HarmonyOS/libs 文件夹复制到你的 HarmonyOS 项目的 entry/libs/ 目录
   ```

2. **配置 build-profile.json5**
   ```json5
   {
     "buildOption": {
       "externalNativeOptions": {
         "abiFilters": ["arm64-v8a"]
       }
     }
   }
   ```

3. **在 ArkTS 中调用**
   ```typescript
   import { nativeModule } from '@kit.ArkTS';

   const tthsdLib = nativeModule.loadLibrary('TTHSD');

   // 启动下载
   const taskId = tthsdLib.start_download(
     '[{"URL":"https://example.com/file.zip","SavePath":"/data/storage/el2/base/haps/entry/files/file.zip","ShowName":"file.zip","ID":"123"}]',
     1, 4, 10, 0, false, "", "", false, false
   );
   ```

详细文档请参阅 [README-HARMONYOS.md](./README-HARMONYOS.md)
