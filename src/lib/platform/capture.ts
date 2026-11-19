// Sharing a window with the website and handing its frames to a worker: the one way the site reads
// the game, used by Now's live reading (live.svelte.ts) and the capture test page. The browser
// asks the player which window to share; the frames go to a worker on this PC and nowhere else.
//
// Two routes, picked by what the browser has, never by its name:
// - Chrome and Edge: a stream reader (MediaStreamTrackProcessor) hands over each frame as the
//   stream delivers it. The stream itself goes to the worker where the browser allows that (Edge
//   refuses), else it is read on the page and each frame handed over.
// - Firefox: no stream reader, so the shared window plays in a video that is never shown and each
//   new frame becomes a VideoFrame. A page the browser counts as hidden (minimised, or covered by
//   the game) gets no frame callbacks, so ticks from a small worker, which browsers don't slow
//   like a hidden page's timers, keep it going.

declare class MediaStreamTrackProcessor {
  constructor(init: { track: MediaStreamTrack });
  readable: ReadableStream<VideoFrame>;
}

/** What the browser offers for reading a shared window, checked by feature rather than by name. */
export const support = {
  share: typeof navigator !== "undefined" && !!navigator.mediaDevices?.getDisplayMedia,
  /** Each frame handed over as the stream delivers it: Chrome and Edge. */
  trackFrames: typeof window !== "undefined" && "MediaStreamTrackProcessor" in window,
  /** Each frame taken from the shared window playing in a video as it arrives: Firefox. */
  videoFrames:
    typeof window !== "undefined" &&
    "VideoFrame" in window &&
    typeof HTMLVideoElement !== "undefined" &&
    "requestVideoFrameCallback" in HTMLVideoElement.prototype,
};

/** Whether this browser can read a shared window at all. */
export const canRead = support.share && (support.trackFrames || support.videoFrames);

/** Why it can't, in a line, for a player; null when it can. */
export const cantRead: string | null = canRead
  ? null
  : `${support.share ? "This browser can share a window but can't hand its frames to the page." : "This browser can't share a window."} Reading the game needs Firefox, Chrome or Edge on a computer.`;

/** The messages a frame-reading worker takes from the page. */
export type FrameMessage = { op: "track"; track: MediaStreamTrack } | { op: "frame"; frame: VideoFrame };

export interface SharedWindow {
  /** The shared window's name, as the browser gives it. */
  label: string;
  /** What was shared: "window", "monitor" or "browser", where the browser says. */
  surface: string;
  /** Where frames are read: in the worker (the stream handed over), on the page, or from a video. */
  readIn: "worker" | "page" | "video";
  /** Why the stream couldn't be handed to the worker, when it couldn't. */
  readInWhy: string | null;
  /** Frames handed over from the page so far (not counted when the worker reads the stream). */
  delivered(): number;
  /** Stops sharing. Safe to call twice. */
  stop(): void;
}

/** How often a hidden page looks for a new frame, where it gets no frame callbacks. */
const HIDDEN_LOOK_MS = 40;

/**
 * Asks the browser for a window to share and starts handing its frames to `send` (a worker's
 * postMessage, transfers included). Null if the player closed the browser's picker. `onended`
 * runs when sharing stops from the browser's side (its own Stop sharing button, or the window
 * closing); not after `stop()`.
 */
export async function shareWindow(
  send: (message: FrameMessage, transfer: Transferable[]) => void,
  onended: () => void,
): Promise<SharedWindow | null> {
  let stream: MediaStream;
  try {
    stream = await navigator.mediaDevices.getDisplayMedia({
      // The window's own size: no scaling to a default resolution.
      video: { displaySurface: "window", frameRate: { ideal: 60 }, width: { ideal: 4096 }, height: { ideal: 2304 } },
      audio: false,
      // Chrome's options for what the picker offers.
      ...({ selfBrowserSurface: "exclude", surfaceSwitching: "include", preferCurrentTab: false } as object),
    });
  } catch (err) {
    if (err instanceof Error && err.name === "NotAllowedError") return null;
    throw err;
  }
  const track = stream.getVideoTracks()[0];
  let stopped = false;
  let delivered = 0;
  const cleanups: (() => void)[] = [];

  const end = () => {
    if (stopped) return;
    shared.stop();
    onended();
  };
  track.addEventListener("ended", end);

  const shared: SharedWindow = {
    label: track.label,
    surface: String((track.getSettings() as { displaySurface?: string }).displaySurface ?? ""),
    readIn: "page",
    readInWhy: null,
    delivered: () => delivered,
    stop() {
      if (stopped) return;
      stopped = true;
      for (const cleanup of cleanups) cleanup();
      track.stop();
    },
  };
  const handOver = (frame: VideoFrame) => {
    delivered++;
    send({ op: "frame", frame }, [frame as unknown as Transferable]);
  };

  if (!support.trackFrames) {
    shared.readIn = "video";
    cleanups.push(readFromVideo(stream, handOver, end));
  } else {
    try {
      send({ op: "track", track }, [track as unknown as Transferable]);
      shared.readIn = "worker";
    } catch (err) {
      shared.readIn = "page";
      shared.readInWhy = err instanceof Error ? `${err.name}: ${err.message}` : String(err);
      const reader = new MediaStreamTrackProcessor({ track }).readable.getReader();
      cleanups.push(() => void reader.cancel());
      void (async () => {
        for (;;) {
          const { value, done } = await reader.read();
          if (done || !value) break;
          if (stopped) value.close();
          else handOver(value);
        }
        end();
      })();
    }
  }
  return shared;
}

/** Firefox's route (see the top of the file). Returns what stops it. */
function readFromVideo(stream: MediaStream, handOver: (frame: VideoFrame) => void, onfail: () => void): () => void {
  const v = document.createElement("video");
  v.muted = true;
  v.playsInline = true;
  v.srcObject = stream;
  let running = true;
  let last = -1;
  const take = (mediaTime: number) => {
    if (!running || v.readyState < 2 || mediaTime === last) return;
    last = mediaTime;
    let frame: VideoFrame;
    try {
      frame = new VideoFrame(v, { timestamp: Math.round(mediaTime * 1e6) });
    } catch {
      return;
    }
    handOver(frame);
  };
  const onFrame = (_now: number, meta: { mediaTime: number }) => {
    take(meta.mediaTime);
    if (running) v.requestVideoFrameCallback(onFrame);
  };
  v.play()
    .then(() => v.requestVideoFrameCallback(onFrame))
    .catch(() => onfail());
  const tick = `setInterval(() => postMessage(0), ${HIDDEN_LOOK_MS});`;
  const url = URL.createObjectURL(new Blob([tick], { type: "text/javascript" }));
  const ticker = new Worker(url);
  ticker.onmessage = () => {
    if (document.visibilityState === "hidden") take(v.currentTime);
  };
  return () => {
    running = false;
    ticker.terminate();
    URL.revokeObjectURL(url);
    v.pause();
    v.srcObject = null;
  };
}

/**
 * The worker's side of the Chrome route where the stream itself was handed over: each frame to
 * `handle` as the stream delivers it, one at a time. Resolves when the stream ends; `cancel`
 * stops it early.
 */
export function followTrack(track: MediaStreamTrack, handle: (frame: VideoFrame) => Promise<void>) {
  const reader = new MediaStreamTrackProcessor({ track }).readable.getReader();
  const done = (async () => {
    for (;;) {
      const { value, done } = await reader.read();
      if (done || !value) break;
      await handle(value);
    }
  })();
  return { done, cancel: () => void reader.cancel() };
}

let canvas: OffscreenCanvas | null = null;
let context: OffscreenCanvasRenderingContext2D | null = null;

/** A frame as RGBA bytes, converted the way any page reading it would. For use in a worker. */
export function frameRgba(frame: VideoFrame): { data: Uint8Array; width: number; height: number } {
  const width = frame.displayWidth;
  const height = frame.displayHeight;
  if (!canvas || canvas.width !== width || canvas.height !== height) {
    canvas = new OffscreenCanvas(width, height);
    context = canvas.getContext("2d", { willReadFrequently: true, alpha: false });
  }
  context!.drawImage(frame, 0, 0);
  const image = context!.getImageData(0, 0, width, height);
  return { data: new Uint8Array(image.data.buffer), width, height };
}
