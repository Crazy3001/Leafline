// The website's pop-out: Chrome and Edge's Document Picture-in-Picture window, which stays over
// other windows (the game included) like the program's pinned Now panel. It shows the session
// being typed in, drawn by this page, so it follows the same numbers.

import { mount, unmount } from "svelte";

interface DocumentPictureInPicture {
  requestWindow(options: { width: number; height: number }): Promise<Window>;
  window: Window | null;
}

let open: { window: Window; app: ReturnType<typeof mount> } | null = null;

export const pipSupported = () => typeof window !== "undefined" && "documentPictureInPicture" in window;

export function pipOpen(): boolean {
  return open !== null;
}

/** Copies the page's styles and look into the new window, so the HUD draws as it does here. */
function dress(target: Window) {
  for (const sheet of Array.from(document.styleSheets)) {
    try {
      const style = target.document.createElement("style");
      style.textContent = Array.from(sheet.cssRules, (rule) => rule.cssText).join("\n");
      target.document.head.append(style);
    } catch {
      if (sheet.href) {
        const link = target.document.createElement("link");
        link.rel = "stylesheet";
        link.href = sheet.href;
        target.document.head.append(link);
      }
    }
  }
  const theme = document.documentElement.dataset.theme;
  if (theme) target.document.documentElement.dataset.theme = theme;
  target.document.title = "Leafline";
  target.document.body.style.margin = "0";
  target.document.body.style.background = "var(--bg)";
}

/** Opens the pop-out, or brings nothing up where the browser can't. `onclose` runs when it closes. */
export async function openPip(onclose: () => void): Promise<boolean> {
  if (open) return true;
  const api = (window as unknown as { documentPictureInPicture?: DocumentPictureInPicture }).documentPictureInPicture;
  if (!api) return false;
  const target = await api.requestWindow({ width: 520, height: 110 });
  dress(target);
  // Loaded here, not at the top: the HUD imports the backend, which this module is part of.
  const { default: Hud } = await import("../components/TypedSessionHud.svelte");
  const app = mount(Hud, { target: target.document.body });
  open = { window: target, app };
  target.addEventListener("pagehide", () => {
    if (open?.window === target) {
      void unmount(app);
      open = null;
      onclose();
    }
  });
  return true;
}

export function closePip() {
  open?.window.close();
}
