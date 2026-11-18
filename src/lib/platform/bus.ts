// The website's events, in place of Tauri's: whatever the worker says changed, and what the page
// sends itself (Go here trips, links between views). Delivered in this page, its picture-in-picture
// window (the same page), and other open tabs of the site.

import type { UnlistenFn } from "@tauri-apps/api/event";

type Handler = (event: { payload: never }) => void;

const handlers = new Map<string, Set<Handler>>();
const channel = typeof BroadcastChannel === "undefined" ? null : new BroadcastChannel("leafline-events");

channel?.addEventListener("message", (message: MessageEvent<{ event: string; payload: unknown }>) => {
  deliver(message.data.event, message.data.payload);
});

function deliver(event: string, payload: unknown) {
  for (const handler of handlers.get(event) ?? []) {
    try {
      handler({ payload: payload as never });
    } catch (err) {
      console.error(`handling ${event}:`, err);
    }
  }
}

export async function listen<T>(event: string, handler: (event: { payload: T }) => void): Promise<UnlistenFn> {
  const set = handlers.get(event) ?? new Set();
  set.add(handler as Handler);
  handlers.set(event, set);
  return () => {
    set.delete(handler as Handler);
  };
}

export async function emit(event: string, payload?: unknown): Promise<void> {
  deliver(event, payload);
  channel?.postMessage({ event, payload: payload === undefined ? null : JSON.parse(JSON.stringify(payload)) });
}
