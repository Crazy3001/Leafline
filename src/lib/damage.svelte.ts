// What Leafline read of the player's damage and stats from the game: the stat window and its
// detail pane, the damaging skills in the skill window, and the buffs the player marks as on. The
// training estimates use it unless the player typed their own. Kept by the backend per character;
// this follows the character being played.

import { backend } from "./api";
import type { Assumptions, DamageSkill, PlayerDamage, StatsView } from "./types";

export const damage = $state<{ name: string | null; value: PlayerDamage | null; stats: StatsView | null }>({
  name: null,
  value: null,
  stats: null,
});

let watching = false;

function askStats(name: string) {
  backend
    .characterStats(name)
    .then((stats) => {
      if (damage.name === name) damage.stats = stats;
    })
    .catch(() => {});
}

/** Follows what the backend has read for `name`, the character being played. */
export function followDamage(name: string | null) {
  if (!watching) {
    watching = true;
    // A build saved in any window (skills typed, gear put on) changes the buffs' levels and the Attack.
    void backend.onBuilds(() => {
      if (damage.name) askStats(damage.name);
    });
    void backend.onPlayerDamage((change) => {
      if (!damage.name || change.name.toLowerCase() !== damage.name.toLowerCase()) return;
      damage.value = change.damage;
      askStats(damage.name);
    });
  }
  if (name === damage.name) return;
  damage.name = name;
  damage.value = null;
  damage.stats = null;
  if (name) {
    backend
      .playerDamage(name)
      .then((value) => {
        if (damage.name === name) damage.value = value;
      })
      .catch(() => {});
    askStats(name);
  }
}

/** Marks a buff of the character being followed as on or off. */
export function setBuff(skill: number, on: boolean): Promise<void> {
  return damage.name ? backend.setBuff(damage.name, skill, on) : Promise.resolve();
}

export interface Effective {
  /** The assumptions to give the estimates, with what was read filling what wasn't typed. */
  assumptions: Assumptions;
  /** The skill counted, when one was read and chosen. */
  skill: DamageSkill | null;
  /** The attack range came from the game, not from the player. */
  attackRead: boolean;
  /** The attack range counts buffs marked on that weren't on when it was read. */
  buffed: boolean;
}

/** The player's typed values over what was read from the game, with buffs marked on counted. */
export function effective(typed: Assumptions, read: PlayerDamage | null, stats: StatsView | null = null): Effective {
  const attackRead = typed.attackMin == null && typed.attackMax == null && read?.attackMin != null && read?.attackMax != null;
  const skills = read?.skills ?? [];
  const skill =
    typed.skillName === "custom"
      ? null
      : (typed.skillName ? skills.find((s) => s.name === typed.skillName) : null) ??
        [...skills].sort((a, b) => b.percent - a.percent)[0] ??
        null;
  // The range with the buffs on now, or without any when none is on: the window's own range can
  // hold a buff that has since run out.
  const now = stats?.buffed ?? stats?.base ?? null;
  const buffedMin = now?.attackMin ?? null;
  const buffedMax = now?.attackMax ?? null;
  const buffed =
    attackRead &&
    stats?.buffed != null &&
    buffedMin != null &&
    buffedMax != null &&
    (buffedMin !== stats.base?.attackMin || buffedMax !== stats.base?.attackMax);
  const useNow = attackRead && buffedMin != null && buffedMax != null;
  return {
    assumptions: {
      ...typed,
      attackMin: attackRead ? (useNow ? buffedMin : read!.attackMin) : typed.attackMin,
      attackMax: attackRead ? (useNow ? buffedMax : read!.attackMax) : typed.attackMax,
      skillFactor: skill ? skill.percent / 100 : typed.skillFactor,
    },
    skill,
    attackRead,
    buffed,
  };
}
