/**
 * TTHSDownloader - TTHSD TypeScript 封装类
 *
 * 封装了对 TTHSD 动态库的所有调用，提供：
 *  - 面向对象的下载器 API
 *  - 内置事件分发（借助 Node.js EventEmitter）
 *  - 生命周期管理（确保 C 回调不被 GC 提前释放）
 *  - Electron + Node.js 双环境兼容
 */

import { EventEmitter } from "events";
import koffi from "koffi";
import * as path from "path";
import { randomUUID } from "crypto";

import {
  DownloadTask,
  DownloadOptions,
  DownloadEvent,
  CallbackData,
  DownloadCallback,
} from "./types";
import { resolveDllPath, loadLibrary, makeNativeCallback } from "./native";

// ------------------------------------------------------------------
// 辅助函数
// ------------------------------------------------------------------

/**
 * 构建任务 JSON 字符串（传给 DLL 第一个参数）
 */
function buildTasksJson(
  urls: string[],
  savePaths: string[],
  showNames?: string[],
  ids?: string[]
): string {
  if (urls.length !== savePaths.length) {
    throw new Error(
      `[TTHSD] urls 与 savePaths 长度不一致: ${urls.length} vs ${savePaths.length}`
    );
  }

  const tasks: DownloadTask[] = urls.map((url, i) => ({
    url,
    save_path: savePaths[i],
    show_name: (showNames?.[i] ?? path.basename(url.split("?")[0])) || `task_${i}`,
    id: ids?.[i] ?? randomUUID(),
  }));

  return JSON.stringify(tasks);
}

// ------------------------------------------------------------------
// 主封装类
// ------------------------------------------------------------------

/**
 * TTHSDownloader
 *
 * 用法示例:
 * ```ts
 * const dl = new TTHSDownloader();
 * const id = dl.startDownload({
 *   urls: ["https://example.com/a.zip"],
 *   savePaths: ["./a.zip"],
 *   callback({ Type, ID }, data) {
 *     if (Type === "update") console.log(data);
 *   },
 * });
 * // 下载完成后资源可以通过 dl.stopDownload(id) 或让库自动清理
 * ```
 *
 * 也可以在构造函数里传入动态库的绝对路径:
 * ```ts
 * const dl = new TTHSDownloader({ dllPath: "/opt/app/libtthsd.so" });
 * ```
 */
export class TTHSDownloader extends EventEmitter {
  private _fn: ReturnType<typeof loadLibrary>;
  /** 保存所有 C 回调引用，防止 GC 回收 */
  private _callbackRefs = new Map<number, koffi.IKoffiRegisteredCallback>();

  constructor(options?: { dllPath?: string }) {
    super();
    const dllPath = resolveDllPath(options?.dllPath);
    this._fn = loadLibrary(dllPath);
  }

  // ----------------------------------------------------------------
  // 私有工具
  // ----------------------------------------------------------------

  private _buildCallbackArgs(callback?: DownloadCallback): {
    cbPtr: unknown;
    cbRef?: koffi.IKoffiRegisteredCallback;
  } {
    if (!callback) return { cbPtr: null };
    const cbRef = makeNativeCallback(callback);
    return { cbPtr: cbRef, cbRef };
  }

  // ----------------------------------------------------------------
  // 公开 API
  // ----------------------------------------------------------------

  /**
   * 创建下载器实例，但**不**立即开始下载。
   * 返回下载器 ID，可传入 startDownloadById / startMultipleDownloadsById。
   */
  getDownloader(
    urls: string[],
    savePaths: string[],
    options?: DownloadOptions
  ): number {
    const {
      threadCount = 64,
      chunkSizeMB = 10,
      callback,
      userAgent,
      useCallbackUrl = false,
      remoteCallbackUrl,
      useSocket,
      showNames,
      ids,
    } = options ?? {};

    const tasksJson = buildTasksJson(urls, savePaths, showNames, ids);
    const { cbPtr, cbRef } = this._buildCallbackArgs(callback);

    const id = this._fn.get_downloader(
      tasksJson,
      urls.length,
      threadCount,
      chunkSizeMB,
      cbPtr,
      useCallbackUrl,
      userAgent ?? null,
      remoteCallbackUrl ?? null,
      useSocket != null ? Buffer.from([useSocket ? 1 : 0]) : null
    ) as number;

    if (id === -1) {
      throw new Error("[TTHSD] getDownloader 失败，DLL 返回 -1");
    }

    if (cbRef) this._callbackRefs.set(id, cbRef);
    return id;
  }

  /**
   * 创建并**立即启动**下载。
   * 返回下载器 ID，可用于暂停 / 恢复 / 停止。
   */
  startDownload(
    urls: string[],
    savePaths: string[],
    options?: DownloadOptions
  ): number {
    const {
      threadCount = 64,
      chunkSizeMB = 10,
      callback,
      userAgent,
      useCallbackUrl = false,
      remoteCallbackUrl,
      useSocket,
      isMultiple,
      showNames,
      ids,
    } = options ?? {};

    const tasksJson = buildTasksJson(urls, savePaths, showNames, ids);
    const { cbPtr, cbRef } = this._buildCallbackArgs(callback);

    const id = this._fn.start_download(
      tasksJson,
      urls.length,
      threadCount,
      chunkSizeMB,
      cbPtr,
      useCallbackUrl,
      userAgent ?? null,
      remoteCallbackUrl ?? null,
      useSocket != null ? Buffer.from([useSocket ? 1 : 0]) : null,
      isMultiple != null ? Buffer.from([isMultiple ? 1 : 0]) : null
    ) as number;

    if (id === -1) {
      throw new Error("[TTHSD] startDownload 失败，DLL 返回 -1");
    }

    if (cbRef) this._callbackRefs.set(id, cbRef);
    return id;
  }

  /** 按 ID 启动顺序下载（需先调用 getDownloader） */
  startDownloadById(downloaderId: number): boolean {
    return (this._fn.start_download_id(downloaderId) as number) === 0;
  }

  /** 按 ID 启动并行下载（需先调用 getDownloader） */
  startMultipleDownloadsById(downloaderId: number): boolean {
    return (this._fn.start_multiple_downloads_id(downloaderId) as number) === 0;
  }

  /** 暂停下载（可通过 resumeDownload 恢复） */
  pauseDownload(downloaderId: number): boolean {
    return (this._fn.pause_download(downloaderId) as number) === 0;
  }

  /** 恢复已暂停的下载（需核心版本 ≥0.5.1） */
  resumeDownload(downloaderId: number): boolean {
    return (this._fn.resume_download(downloaderId) as number) === 0;
  }

  /** 停止下载并销毁下载器实例（无法恢复） */
  stopDownload(downloaderId: number): boolean {
    const ret = (this._fn.stop_download(downloaderId) as number) === 0;
    // 释放 C 回调引用
    const cbRef = this._callbackRefs.get(downloaderId);
    if (cbRef) {
      koffi.unregister(cbRef);
      this._callbackRefs.delete(downloaderId);
    }
    return ret;
  }

  /** 释放所有资源（一般在程序退出前调用） */
  dispose(): void {
    for (const [, ref] of this._callbackRefs) {
      koffi.unregister(ref);
    }
    this._callbackRefs.clear();
  }
}
