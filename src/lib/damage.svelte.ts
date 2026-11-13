// What Leafline read of the player's damage from the game: the stat window's attack range and the
// damaging skills in the skill window. The training estimates use it unless the player typed their
// own. Kept by the backend per character; this follows the character being played.

import { backend } from "./api";
import type { Assumptions, DamageSkill, PlayerDamage } from "./types";

export const damage = $state<{ name: string | null; value: PlayerDamage | null }>({ name: null, value: null });

let watching = false;

/** Follows what the backend has read for `name`, the character being played. */
export function followDamage(name: string | null) {
  if (!watching) {
    watching = true;
    void backend.onPlayerDamage((change) => {
      if (damage.name && change.name.toLowerCase() === damage.name.toLowerCase()) damage.value = change.damage;
    });
  }
  if (name === damage.name) return;
  damage.name = name;
  damage.value = null;
  if (name) {
    backend
      .playerDamage(name)
      .then((value) => {
        if (damage.name === name) damage.value = value;
      })
      .catch(() => {});
  }
}

export interface Effective {
  /** The assumptions to give the estimates, with what was read filling what wasn't typed. */
  assumptions: Assumptions;
  /** The skill counted, when one was read and chosen. */
  skill: DamageSkill | null;
  /** The attack range came from the game, not from the player. */
  attackRead: boolean;
}

/** The player's typed values over what was read from the game. */
export function effective(typed: Assumptions, read: PlayerDamage | null): Effective {
  const attackRead = typed.attackMin == null && typed.attackMax == null && read?.attackMin != null && read?.attackMax != null;
  const skills = read?.skills ?? [];
  const skill =
    typed.skillName === "custom"
      ? null
      : (typed.skillName ? skills.find((s) => s.name === typed.skillName) : null) ??
        [...skills].sort((a, b) => b.percent - a.percent)[0] ??
        null;
  return {
    assumptions: {
      ...typed,
      attackMin: attackRead ? read!.attackMin : typed.attackMin,
      attackMax: attackRead ? read!.attackMax : typed.attackMax,
      skillFactor: skill ? skill.percent / 100 : typed.skillFactor,
    },
    skill,
    attackRead,
  };
}
