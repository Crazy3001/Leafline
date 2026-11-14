// Every skill in the map data, for skill cards and Lookup → Skills: read once from the build
// catalog, which the backend already builds from the map data.
import { backend } from "./api";
import type { SkillInfo } from "./types";

export const skillData = $state<{ skills: SkillInfo[]; loaded: boolean }>({ skills: [], loaded: false });

let asked: Promise<void> | null = null;

/** Loads the skills the first time something asks; later calls wait on the same load. */
export function loadSkills(): Promise<void> {
  asked ??= backend
    .buildCatalog()
    .then((catalog) => {
      skillData.skills = catalog?.skills ?? [];
    })
    .catch(() => {
      asked = null;
    })
    .finally(() => (skillData.loaded = true));
  return asked;
}

export function skillById(id: number | null | undefined): SkillInfo | null {
  return id == null ? null : (skillData.skills.find((s) => s.id === id) ?? null);
}

export function skillNamed(name: string | null | undefined): SkillInfo | null {
  const wanted = name?.trim().toLowerCase();
  return wanted ? (skillData.skills.find((s) => s.name.toLowerCase() === wanted) ?? null) : null;
}

/** The skill Lookup → Skills has open. */
export const skillView = $state<{ id: number | null }>({ id: null });
