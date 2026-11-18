// The program's backend: the shared command table over Tauri.

import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";

import { commandBackend } from "./commands";

export const tauriBackend = commandBackend(invoke, listen, emit);
