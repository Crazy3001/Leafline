// Windows over the game don't take focus when clicked, so play goes on. A box being typed in is
// the exception: the window takes focus while it is in use, then lets go.

import { backend } from "./api";

/** Elements that need the keyboard. */
function typesText(el: EventTarget | null): boolean {
  if (!(el instanceof HTMLElement)) return false;
  if (el.isContentEditable || el instanceof HTMLTextAreaElement || el instanceof HTMLSelectElement) return true;
  if (!(el instanceof HTMLInputElement)) return false;
  return !["checkbox", "radio", "range", "button", "submit", "reset", "color", "file"].includes(el.type);
}

/** Starts following focus in this window. Returns a function that stops it. */
export function watchTyping(): () => void {
  let typing = false;
  const set = (next: boolean) => {
    if (next === typing) return;
    typing = next;
    backend.setTyping(next).catch(() => {});
  };
  const focusIn = (e: FocusEvent) => {
    if (typesText(e.target)) set(true);
  };
  const focusOut = (e: FocusEvent) => {
    if (!typesText(e.relatedTarget)) set(false);
  };
  // Escape leaves the box, giving the keys back.
  const key = (e: KeyboardEvent) => {
    if (e.key === "Escape" && typesText(document.activeElement)) (document.activeElement as HTMLElement).blur();
  };
  document.addEventListener("focusin", focusIn);
  document.addEventListener("focusout", focusOut);
  document.addEventListener("keydown", key);
  return () => {
    document.removeEventListener("focusin", focusIn);
    document.removeEventListener("focusout", focusOut);
    document.removeEventListener("keydown", key);
    set(false);
  };
}
