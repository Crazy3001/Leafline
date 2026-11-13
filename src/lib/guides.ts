// Written guides for each job on other sites, linked from the build planner. The ready-made plans
// in src-tauri/src/build/starters.json follow NiaDB's guides and credit the exact page; nothing
// else is copied from either site.

const NIADB = "https://meowdb.com/msclassic/guides";
const MAPLESCANNER = "https://maplescanner.com/classic/gms/en/build";

export interface GuideLink {
  site: string;
  levels: string;
  url: string;
}

/** The two sites' page names for a job, by job id. */
const PAGES: Record<number, { niadb: string; scanner: string; levels: { niadb: string; scanner: string } }> = {
  100: { niadb: "warrior", scanner: "warrior", levels: { niadb: "Lv 1-30", scanner: "Lv 10-30" } },
  200: { niadb: "magician", scanner: "magician", levels: { niadb: "Lv 1-30", scanner: "Lv 10-30" } },
  300: { niadb: "bowman", scanner: "bowman", levels: { niadb: "Lv 1-30", scanner: "Lv 10-30" } },
  400: { niadb: "thief", scanner: "thief", levels: { niadb: "Lv 1-30", scanner: "Lv 10-30" } },
  110: { niadb: "fighter", scanner: "fighter", levels: { niadb: "Lv 30-70", scanner: "Lv 1-70" } },
  120: { niadb: "page", scanner: "page", levels: { niadb: "Lv 30-70", scanner: "Lv 1-70" } },
  130: { niadb: "spearman", scanner: "spearman", levels: { niadb: "Lv 30-70", scanner: "Lv 1-70" } },
  210: { niadb: "fp-wizard", scanner: "fp", levels: { niadb: "Lv 30-70", scanner: "Lv 1-70" } },
  220: { niadb: "il-wizard", scanner: "il", levels: { niadb: "Lv 30-70", scanner: "Lv 1-70" } },
  230: { niadb: "cleric", scanner: "cleric", levels: { niadb: "Lv 30-70", scanner: "Lv 1-70" } },
  310: { niadb: "hunter", scanner: "hunter", levels: { niadb: "Lv 30-70", scanner: "Lv 1-70" } },
  320: { niadb: "crossbowman", scanner: "crossbowman", levels: { niadb: "Lv 30-70", scanner: "Lv 1-70" } },
  410: { niadb: "assassin", scanner: "assassin", levels: { niadb: "Lv 30-70", scanner: "Lv 1-70" } },
  420: { niadb: "bandit", scanner: "bandit", levels: { niadb: "Lv 30-70", scanner: "Lv 1-70" } },
};

export function guideLinks(job: number): GuideLink[] {
  const page = PAGES[job];
  if (!page) return [];
  return [
    { site: "MapleScanner", levels: page.levels.scanner, url: `${MAPLESCANNER}/${page.scanner}/` },
    { site: "NiaDB", levels: page.levels.niadb, url: `${NIADB}/${page.niadb}-class-guide` },
  ];
}
