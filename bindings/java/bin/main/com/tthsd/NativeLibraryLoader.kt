package com.tthsd

import java.io.File
import java.io.FileOutputStream

/**
 * NativeLibraryLoader
 *
 * 负责自动解析并加载 TTHSD 动态库。
 *
 * 工作流程：
 *   1. 优先检测 TTHSD_LIB_PATH 环境变量
 *   2. 从 JAR 内部 /native/<os>/<arch>/ 提取动态库到临时目录
 *   3. 从当前工作目录、程序目录搜索
 *
 * 这使得将 TTHSD 发布为单一 fat-jar （内嵌动态库）成为可能。
 * 在 Minecraft Mod、Plugin 或第三方启动器中均可直接引用该 jar。
 */
object NativeLibraryLoader {

    private val osName: String = System.getProperty("os.name").lowercase()
    private val archName: String = System.getProperty("os.arch").lowercase()

    /** 动态库文件名（根据 OS 自动选择） */
    val libFileName: String
        get() = when {
            osName.contains("win")   -> "tthsd.dll"
            osName.contains("mac")   -> "tthsd.dylib"
            osName.contains("android")   -> "tthsd_android.so"
            else                     -> "tthsd.so"
        }

    /** OS 分类标识（对应 JAR 内路径 /native/<osKey>/） */
    private val osKey: String
        get() = when {
            osName.contains("win")   -> "windows"
            osName.contains("mac")   -> "macos"
            osName.contains("android")   ->  "android"
            else                     -> "linux"
        }

    /** CPU 架构（对应 JAR 内路径 /native/<osKey>/<archKey>/） */
    private val archKey: String
        get() = when {
            archName.contains("aarch64") || archName.contains("arm64") -> "arm64"
            archName.contains("arm")                                    -> "arm"
            archName.contains("x86_64") || archName.contains("amd64")  -> "x86_64"
            archName.contains("x86")                                    -> "x86"
            else                                                        -> archName
        }

    /**
     * 解析动态库绝对路径，如有需要会将其从 JAR 内部提取到临时目录。
     *
     * @return 动态库绝对路径（供 JNA.Native.load 使用）
     */
    fun resolve(): String {
        // 1. 环境变量
        val envPath = System.getenv("TTHSD_LIB_PATH")
        if (envPath != null && File(envPath).exists()) return envPath

        // 2. 从 JAR 内部提取
        val extracted = extractFromJar()
        if (extracted != null) return extracted.absolutePath

        // 3. 当前工作目录、主类所在目录
        val candidates = listOf(
            File(System.getProperty("user.dir"), libFileName),
            File(
                NativeLibraryLoader::class.java.protectionDomain
                    ?.codeSource?.location?.toURI()?.let { File(it).parentFile }
                    ?: File("."),
                libFileName
            )
        )

        for (f in candidates) {
            if (f.exists()) return f.absolutePath
        }

        throw UnsatisfiedLinkError(
            "[TTHSD] 未能找到动态库 $libFileName，请设置 TTHSD_LIB_PATH 环境变量，" +
            "或将动态库放置到工作目录中。"
        )
    }

    /**
     * 尝试从 JAR 内部 /native/\<os>/\<arch>/\<libFileName> 提取到 Java 临时目录。
     */
    private fun extractFromJar(): File? {
        val resourcePath = "/native/$osKey/$archKey/$libFileName"
        val inputStream = NativeLibraryLoader::class.java.getResourceAsStream(resourcePath)
            ?: return null

        val tempDir = File(System.getProperty("java.io.tmpdir"), "tthsd_native")
        tempDir.mkdirs()

        val outFile = File(tempDir, libFileName)
        // 若已提取且文件完整则直接复用
        if (outFile.exists() && outFile.length() > 0) return outFile

        FileOutputStream(outFile).use { out ->
            inputStream.copyTo(out)
        }
        return outFile
    }
}
