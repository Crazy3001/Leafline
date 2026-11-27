// Each character's look: the picture of them standing in the game's Character Info window, cut out
// by the live reading only when the window named the HUD's own character (records
// `character_looks`). Kept on this PC: it moves only in the records file, and never goes on a share
// card. Shown wherever the character's placeholder stood, dated like other last-known values;
// with none kept, or the Settings switch off, the placeholder stays.

import { backend } from "./api";
import { display } from "./settings.svelte";
import type { CharacterLook } from "./types";

const keyOf = (name: string) => name.trim().toLowerCase();

const kept = $state<{ looks: CharacterLook[] }>({ looks: [] });

async function load() {
  try {
    kept.looks = await backend.characterLooks();
  } catch {
    kept.looks = [];
  }
}

let watching = false;

/** Loads the kept looks and follows changes (a new look read, or forgotten). Safe to call often. */
export function watchLooks() {
  if (watching) return;
  watching = true;
  void load();
  void backend.onKnownCharacter(() => void load());
}

/** `name`'s kept look, or null with none kept or the switch off. */
export function lookOf(name: string | null | undefined): CharacterLook | null {
  if (!name || display.live.characterLook === false) return null;
  const key = keyOf(name);
  return kept.looks.find((l) => keyOf(l.name) === key) ?? null;
}

/** Whether any look is kept, for Settings' "Forget saved look". */
export function anyLook(): boolean {
  return kept.looks.length > 0;
}

/** Forgets every kept look. */
export async function forgetLooks() {
  await backend.forgetLooks();
  kept.looks = [];
}

/** The one-line tip where a placeholder stands for the character. */
export const LOOK_TIP = "Open Character Info in game to use your character's look.";
