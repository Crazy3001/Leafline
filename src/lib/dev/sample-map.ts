// A small made-up world for designing the map card in a plain browser. None of it is real game
// data: the repo must never hold extracted map, monster or quest records.

import type {
  Guide,
  Hunt,
  HuntingGround,
  MapDetails,
  MapPage,
  MapQuery,
  MapRef,
  MapRoute,
  MonsterDrops,
  MonsterDetails,
  MonsterQuery,
  MonsterPage,
  MonsterRow,
  MonsterSheet,
  NpcPage,
  NpcQuery,
  NpcSheet,
  ObjectiveGuide,
  QuestQuery,
  TrainingSpots,
} from "../types";

type Place = Omit<MapDetails, "map" | "links" | "returnMap" | "mobTotal" | "expPerClear" | "levelRange"> & {
  name: string;
  street: string;
  returnTo: number | null;
};

const places: Record<number, Place> = {
  1: {
    name: "Mossbank",
    street: "Fern Road",
    region: "Fern Isle",
    isTown: true,
    returnTo: null,
    mobs: [],
    npcs: [
      { id: 50, name: "Warden Ilse", quests: [{ id: 70, name: "Shell Sorting", levelMin: 3 }] },
      { id: 52, name: "Tinker Rolf", quests: [] },
    ],
    quests: [],
    exits: [],
  },
  2: {
    name: "Mossbank Meadow I",
    street: "Fern Road",
    region: "Fern Isle",
    isTown: false,
    returnTo: 1,
    mobs: [
      { id: 10, name: "Pebble Snail", level: 2, hp: 20, exp: 4, count: 9, boss: false, undead: false },
      { id: 11, name: "Moss Slime", level: 5, hp: 55, exp: 9, count: 12, boss: false, undead: false },
      { id: 13, name: "Thistle Sprite", level: 7, hp: 90, exp: 14, count: 4, boss: false, undead: false },
    ],
    npcs: [{ id: 53, name: "Lost Courier", quests: [{ id: 72, name: "Undelivered Letters", levelMin: 6 }] }],
    quests: [
      {
        quest: { id: 71, name: "Slime Samples", levelMin: 1 },
        giver: "Innkeeper Bo",
        targets: [{ mobId: 11, name: "Moss Slime", count: 25 }],
      },
      {
        quest: { id: 70, name: "Shell Sorting", levelMin: 3 },
        giver: "Warden Ilse",
        targets: [{ mobId: 10, name: "Pebble Snail", count: 40 }],
      },
    ],
    exits: [],
  },
  3: {
    name: "Mossbank Meadow II",
    street: "Fern Road",
    region: "Fern Isle",
    isTown: false,
    returnTo: 1,
    mobs: [{ id: 11, name: "Moss Slime", level: 5, hp: 55, exp: 9, count: 18, boss: false, undead: false }],
    npcs: [],
    quests: [],
    exits: [],
  },
  4: {
    name: "Burrow Den",
    street: "Fern Road",
    region: "Fern Isle",
    isTown: false,
    returnTo: 1,
    mobs: [{ id: 12, name: "Burrow Rat", level: 8, hp: 120, exp: 15, count: 10, boss: false, undead: false }],
    npcs: [],
    quests: [],
    exits: [],
  },
  5: {
    name: "Mossbank Inn",
    street: "Fern Road",
    region: "Fern Isle",
    isTown: false,
    returnTo: 1,
    mobs: [],
    npcs: [{ id: 51, name: "Innkeeper Bo", quests: [{ id: 71, name: "Slime Samples", levelMin: 1 }] }],
    quests: [],
    exits: [],
  },
  6: {
    name: "Old Ferry Landing",
    street: "Fern Road",
    region: "Fern Isle",
    isTown: false,
    returnTo: 1,
    mobs: [],
    npcs: [],
    quests: [],
    exits: [],
  },
};

const links: [number, MapDetails["exits"][number]["direction"], number, string, boolean][] = [
  [1, "east", 2, "east00", false],
  [1, "in", 5, "in00", false],
  [1, "west", 6, "west00", false],
  [2, "west", 1, "west00", false],
  [2, "east", 3, "east00", false],
  [2, "in", 4, "h_in00", true],
  [2, "other", 99, "tp01", false],
  [3, "west", 2, "west00", false],
  [4, "out", 2, "out00", false],
  [5, "out", 1, "out00", false],
  [6, "east", 1, "east00", false],
];

function ref(id: number): MapRef {
  const place = places[id];
  return place
    ? { id, name: place.name, street: place.street, known: true }
    : { id, name: "Unknown map", street: null, known: false };
}

export function sampleCard(id: number): MapDetails | null {
  const place = places[id];
  if (!place) return null;
  const levels = place.mobs.map((m) => m.level);
  return {
    ...place,
    map: ref(id),
    returnMap: place.returnTo == null ? null : ref(place.returnTo),
    levelRange: levels.length ? [Math.min(...levels), Math.max(...levels)] : null,
    mobTotal: place.mobs.reduce((sum, m) => sum + m.count, 0),
    expPerClear: place.mobs.reduce((sum, m) => sum + m.exp * m.count, 0),
    exits: links
      .filter(([from]) => from === id)
      .map(([, direction, to, portal, hidden]) => ({
        direction,
        side: null,
        height: null,
        to: ref(to),
        portals: [portal],
        hidden,
      })),
    links: [
      { label: "mapleclassic.wiki", url: "https://mapleclassic.wiki" },
      { label: "MeowDB", url: "https://meowdb.com/msclassic/maps" },
      { label: "henesys.gg", url: "https://henesys.gg/maps" },
    ],
  };
}

export function sampleSearch(text: string): MapRef[] {
  const query = text.trim().toLowerCase();
  if (!query) return [];
  return Object.keys(places)
    .map(Number)
    .filter((id) => places[id].name.toLowerCase().includes(query))
    .map(ref);
}

export function sampleRoute(from: number, to: number): MapRoute | null {
  const cameFrom = new Map<number, (typeof links)[number]>();
  const queue = [from];
  const seen = new Set([from]);
  while (queue.length) {
    const at = queue.shift()!;
    if (at === to) break;
    for (const link of links.filter(([f, , t]) => f === at && places[t])) {
      if (seen.has(link[2])) continue;
      seen.add(link[2]);
      cameFrom.set(link[2], link);
      queue.push(link[2]);
    }
  }
  if (from !== to && !cameFrom.has(to)) return null;
  const steps: MapRoute["steps"] = [];
  for (let at = to; cameFrom.has(at); ) {
    const [f, direction, t, portal, hidden] = cameFrom.get(at)!;
    steps.unshift({ from: ref(f), to: ref(t), direction, side: null, portal, hidden, travel: null });
    at = f;
  }
  return { from: ref(from), to: ref(to), steps, walking: null };
}

export function sampleDrops(monsters: string[]): MonsterDrops[] {
  return monsters.map((monster, i) => ({
    monster,
    sampleKills: i === 2 ? null : 120 + i * 40,
    gameVersion: "0.1.0",
    fetchedAt: Date.now() / 1000,
    drops:
      i === 2
        ? []
        : [
            {
              item: "Meso",
              category: "meso",
              rate: 68.3,
              rateLow: 59.4,
              rateHigh: 76.1,
              minQuantity: 3,
              maxQuantity: 6,
              sampleKills: 120 + i * 40,
              sampleDrops: 82,
              gameVersion: "0.1.0",
              quest: false,
            },
            {
              item: `${monster.split(" ").pop()} Fragment`,
              category: "etc",
              rate: 44.2,
              rateLow: 35.6,
              rateHigh: 53.1,
              minQuantity: null,
              maxQuantity: null,
              sampleKills: 120 + i * 40,
              sampleDrops: 53,
              gameVersion: "0.1.0",
              quest: false,
            },
            {
              item: "Green Tonic",
              category: "use",
              rate: 2.5,
              rateLow: 0.9,
              rateHigh: 7.1,
              minQuantity: null,
              maxQuantity: null,
              sampleKills: 120 + i * 40,
              sampleDrops: 3,
              gameVersion: "0.1.0",
              quest: false,
            },
            {
              item: "Fern Ribbon",
              category: "equip",
              rate: null,
              rateLow: null,
              rateHigh: null,
              minQuantity: null,
              maxQuantity: null,
              sampleKills: null,
              sampleDrops: null,
              gameVersion: null,
              quest: false,
            },
          ],
  }));
}

/** Item names that look like collected items rather than monsters, for the mock guide. */
const ITEMISH = /\b(tail|shell|leaf|candies|anthurium|fragment|whisker|wing)\b/i;
/** Items the mock wiki has no records for, to show that case. */
const UNLOGGED = /\b(candies|anthurium)\b/i;

function ground(id: number, count: number, from: number | null): HuntingGround {
  const route = from == null ? null : sampleRoute(from, id);
  // The sample spots' measured rate on map 2, as kills of this monster.
  const perClear = sampleCard(id)?.expPerClear ?? 0;
  const pace = id === 2 && perClear ? { expPerHour: 1840, activeSeconds: 1260, killsPerHour: (1840 * count) / perClear } : null;
  const mapTotal = places[id].mobs.reduce((sum, m) => sum + m.count, 0);
  return { map: ref(id), count, mapTotal, hops: route ? route.steps.length : null, travel: null, pace };
}

function hunt(id: number, from: number | null, rate: number | null): Hunt {
  const mob = Object.values(places)
    .flatMap((p) => p.mobs)
    .find((m) => m.id === id)!;
  const maps = Object.entries(places)
    .flatMap(([mapId, p]) => p.mobs.filter((m) => m.id === id).map((m) => ground(Number(mapId), m.count, from)))
    .sort((a, b) => b.count - a.count);
  return {
    monster: { name: mob.name, known: true, level: mob.level, hp: mob.hp, exp: mob.exp, boss: mob.boss },
    drop:
      rate == null
        ? null
        : { rate, rateLow: rate * 0.8, rateHigh: rate * 1.2, sampleKills: 240, gameVersion: "0.1.0", quest: true },
    maps,
  };
}

/** A made-up guide in the guide's shape: monsters from the sample world, whatever was read. */
export function sampleGuide(quests: QuestQuery[], from: number | null, hasDrops: boolean): Guide {
  return {
    from: from == null ? null : ref(from),
    quests: quests.map((q, i) => ({
      title: q.title,
      quest:
        i === 0
          ? {
              quest: { id: 71, name: q.title, levelMin: 1 },
              giver: "Innkeeper Bo",
              giverMaps: [ref(5)],
              turnIn: null,
              turnInMaps: [],
            }
          : null,
      objectives: q.objectives.map((target): ObjectiveGuide => {
        if (!ITEMISH.test(target)) return { target, kind: "monster", item: null, lookedUp: false, hunts: [hunt(11, from, null)] };
        const logged = hasDrops && !UNLOGGED.test(target);
        return {
          target,
          kind: "item",
          item: logged ? target : null,
          lookedUp: hasDrops,
          hunts: logged ? [hunt(12, from, 47.5), hunt(11, from, 6.2)] : [],
        };
      }),
    })),
  };
}

const noDetails: MonsterDetails = {
  mp: 30,
  attack: 14,
  magic: null,
  weaponDef: 0,
  magicDef: 10,
  accuracy: 22,
  evasion: 4,
  speed: -30,
  knockback: 25,
  hpRecovery: null,
  mpRecovery: null,
  aggressive: false,
  invincible: false,
  elements: [],
  magicAttacks: false,
  attackElements: [],
  casts: [],
};

/** Beyond the monsters the sample maps have: a boss, an undead and one on no map. */
const bestiary: (MonsterRow & { details: MonsterDetails; region: string | null })[] = [
  ...[...new Map(Object.values(places).flatMap((p) => p.mobs).map((m) => [m.id, m])).values()].map((m) => ({
    id: m.id,
    name: m.name,
    level: m.level,
    hp: m.hp,
    exp: m.exp,
    boss: m.boss,
    undead: m.undead,
    note: null,
    region: "Fern Isle",
    details: noDetails,
  })),
  {
    id: 20,
    name: "Bog Ghoul",
    level: 14,
    hp: 310,
    exp: 24,
    boss: false,
    undead: true,
    note: null,
    region: "Fern Isle",
    details: { ...noDetails, attack: 42, magic: 51, magicAttacks: true, elements: [{ element: "Holy", effect: "Weak" }, { element: "Poison", effect: "Immune" }] },
  },
  {
    id: 21,
    name: "Ember Golem",
    level: 40,
    hp: 34500,
    exp: 340,
    boss: true,
    undead: false,
    note: null,
    region: "Ashlands",
    details: {
      ...noDetails,
      mp: 200,
      attack: 532,
      magic: 585,
      weaponDef: 15,
      magicDef: 15,
      accuracy: 130,
      evasion: 29,
      speed: 5,
      knockback: 2750,
      hpRecovery: 344,
      mpRecovery: 20,
      aggressive: true,
      elements: [{ element: "Ice", effect: "Weak" }, { element: "Fire", effect: "Immune" }],
      attackElements: ["Fire"],
      magicAttacks: true,
      casts: ["debuff", "self_buff"],
    },
  },
  { id: 23, name: "Cinder Bat", level: 28, hp: 640, exp: 52, boss: false, undead: false, note: null, region: "Ashlands", details: noDetails },
  { id: 22, name: "Thorn Vine", level: 18, hp: 520, exp: 31, boss: false, undead: false, note: null, region: null, details: noDetails },
];

/** A rough stand-in for the monster search, only for designing the pages. */
export function sampleMonsters(query: MonsterQuery): MonsterPage {
  const text = query.text.trim().toLowerCase();
  const found = bestiary
    .filter(
      (m) =>
        (!query.kind || (query.kind === "boss" ? m.boss : query.kind === "undead" ? m.undead : !m.boss)) &&
        (!query.weakTo || m.details.elements.some((e) => e.effect === "Weak" && e.element === query.weakTo)) &&
        (!query.region || m.region === query.region) &&
        (query.levelMin == null || m.level >= query.levelMin) &&
        (query.levelMax == null || m.level <= query.levelMax) &&
        (text ? m.name.toLowerCase().includes(text) : m.region != null),
    )
    .sort((a, b) =>
      query.sort === "name"
        ? a.name.localeCompare(b.name)
        : query.sort === "hp"
          ? b.hp - a.hp
          : query.sort === "exp"
            ? b.exp - a.exp
            : query.sort === "efficiency"
              ? b.exp / b.hp - a.exp / a.hp
              : query.near != null
                ? Math.abs(a.level - query.near) - Math.abs(b.level - query.near)
                : a.level - b.level,
    );
  return { monsters: found.slice(query.offset, query.offset + 40).map(({ details: _, region: __, ...row }) => row), total: found.length, closest: false };
}

/** A made-up sheet in the sheet's shape. */
export function sampleMonsterSheet(id: number, from: number | null, hasDrops: boolean): MonsterSheet | null {
  const monster = bestiary.find((m) => m.id === id);
  if (!monster) return null;
  const { details, name, level, hp, exp, boss, undead } = monster;
  const maps = Object.entries(places)
    .flatMap(([mapId, p]) => p.mobs.filter((m) => m.id === id).map((m) => ground(Number(mapId), m.count, from)))
    .sort((a, b) => b.count - a.count);
  return {
    id,
    name,
    level,
    hp,
    exp,
    boss,
    undead,
    details,
    respawn: boss ? 3600 : null,
    maps,
    quests:
      id === 11
        ? [
            { quest: { id: 71, name: "Slime Samples", levelMin: 1 }, giver: "Innkeeper Bo", count: 25 },
            { quest: { id: 72, name: "Undelivered Letters", levelMin: 6 }, giver: "Lost Courier", count: 10 },
          ]
        : [],
    versions: id === 11 ? [{ id: 12, level: 3, hp: 30, exp: 2, maps: [ref(4)] }] : [],
    splits: boss ? [{ id: 11, name: "Moss Slime", count: 6 }] : [],
    drops: hasDrops ? sampleDrops(id === 13 ? ["", "", name] : [name])[id === 13 ? 2 : 0] : null,
  };
}

/** A rough stand-in for the map search, only for designing the pages. */
export function sampleMaps(query: MapQuery): MapPage {
  const text = query.text.trim().toLowerCase();
  const maps = Object.keys(places)
    .map(Number)
    .map((id) => sampleCard(id)!)
    .filter(
      (card) =>
        (!query.kind || (query.kind === "town") === card.isTown) &&
        (!text || card.map.name.toLowerCase().includes(text)) &&
        (query.levelMin == null || (card.levelRange != null && card.levelRange[1] >= query.levelMin)) &&
        (query.levelMax == null || (card.levelRange != null && card.levelRange[0] <= query.levelMax)),
    )
    .map((card) => ({ map: card.map, region: card.region, isTown: card.isTown, levelRange: card.levelRange, note: null }));
  return { maps: maps.slice(query.offset, query.offset + 40), total: maps.length, closest: false };
}

const npcs = Object.entries(places).flatMap(([id, p]) => p.npcs.map((npc) => ({ ...npc, mapId: Number(id) })));

/** A rough stand-in for the NPC search, only for designing the pages. */
export function sampleNpcs(query: NpcQuery): NpcPage {
  const text = query.text.trim().toLowerCase();
  const found = npcs
    .filter((n) => (!text || n.name.toLowerCase().includes(text)) && (query.kind !== "quests" || n.quests.length > 0))
    .map((n) => ({ id: n.id, name: n.name, map: places[n.mapId].name, maps: 1, quests: n.quests.length, note: null }));
  return { npcs: found.slice(query.offset, query.offset + 40), total: found.length, closest: false };
}

/** A made-up sheet in the sheet's shape. */
export function sampleNpcSheet(id: number, from: number | null): NpcSheet | null {
  const npc = npcs.find((n) => n.id === id);
  if (!npc) return null;
  const route = from == null ? null : sampleRoute(from, npc.mapId);
  return {
    id,
    name: npc.name,
    maps: [{ map: ref(npc.mapId), hops: route ? route.steps.length : null, travel: null }],
    quests: npc.quests.map((q) => ({ id: q.id, name: q.name, region: "Fern Isle", levelMin: q.levelMin, giver: npc.name, repeat: null, exp: 120, mesos: 0 })),
    takes: [],
  };
}

/** Every sample map with monsters, as the training spot finder lists them. */
export function sampleSpots(from: number | null): TrainingSpots {
  const spots = Object.keys(places)
    .map(Number)
    .filter((id) => !places[id].isTown && places[id].mobs.length)
    .map((id) => {
      const card = sampleCard(id)!;
      const total = card.mobTotal || 1;
      const route = from == null ? null : sampleRoute(from, id);
      return {
        map: card.map,
        region: card.region,
        levelRange: card.levelRange!,
        averageLevel: card.mobs.reduce((sum, m) => sum + m.level * m.count, 0) / total,
        mobTotal: card.mobTotal,
        expPerClear: card.expPerClear,
        hpPerClear: card.mobs.reduce((sum, m) => sum + m.hp * m.count, 0),
        mobs: card.mobs,
        hops: route ? route.steps.length : null,
        travel: null,
        reachable: true,
        measured: id === 2 ? { expPerHour: 1840, activeSeconds: 1260 } : null,
      };
    });
  return { from: from == null ? null : ref(from), spots };
}
