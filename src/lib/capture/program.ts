// The program's half of the capture test (src-tauri/src/app/capturetest.rs). A developer check,
// so it calls the program straight rather than through the Backend every page shares.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { Agreement, Measure, RecorderStatus, WindowInfo } from "./types";

export const openCaptureTest = () => invoke<void>("open_capture_test");
export const openPattern = () => invoke<void>("open_capture_pattern");

/** The pattern's RGBA bytes for this window. */
export async function patternPixels(info: WindowInfo): Promise<Uint8ClampedArray> {
  const bytes = await invoke<ArrayBuffer>("capture_pattern", {
    width: info.width,
    height: info.height,
    scaleMilli: info.scaleMilli,
  });
  return new Uint8ClampedArray(bytes);
}

export const checkPattern = () => invoke<Measure>("check_capture_pattern");
export const startRecording = (folder: string, perSecond: number) =>
  invoke<RecorderStatus>("start_capture_recording", { folder, perSecond });
export const stopRecording = () => invoke<RecorderStatus>("stop_capture_recording");
export const recordingStatus = () => invoke<RecorderStatus>("capture_recording_status");
export const compareFrames = (folder: string) => invoke<Agreement>("compare_capture_frames", { folder });

export function onCompareProgress(heard: (done: number, total: number) => void): Promise<UnlistenFn> {
  return listen<[number, number]>("capture-compare", (event) => heard(event.payload[0], event.payload[1]));
}

export async function pickFolder(): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({ directory: true, title: "Folder for the capture test's frames" });
  return typeof picked === "string" ? picked : null;
}
