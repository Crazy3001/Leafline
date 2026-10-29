// A small made-up world for designing the map card in a plain browser. None of it is real game
// data: the repo must never hold extracted map, monster or quest records.

import type {
  Guide,
  Hunt,
  HuntingGround,
  MapDetails,
  MapRef,
  MapRoute,
  MonsterDrops,
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
      .map(([, direction, to, portal, hidden]) => ({ direction, to: ref(to), portals: [portal], hidden })),
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
    steps.unshift({ from: ref(f), to: ref(t), direction, portal, hidden });
    at = f;
  }
  return { from: ref(from), to: ref(to), steps };
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
  return { map: ref(id), count, hops: route ? route.steps.length : null };
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
export function sampleGuide(quests: QuestQuery[], from: number | null, lookedUp: Set<string>): Guide {
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
            }
          : null,
      objectives: q.objectives.map((target): ObjectiveGuide => {
        if (!ITEMISH.test(target)) return { target, kind: "monster", item: null, lookedUp: false, hunts: [hunt(11, from, null)] };
        const asked = lookedUp.has(target);
        const logged = asked && !UNLOGGED.test(target);
        return {
          target,
          kind: "item",
          item: logged ? target : null,
          lookedUp: asked,
          hunts: logged ? [hunt(12, from, 47.5), hunt(11, from, 6.2)] : [],
        };
      }),
    })),
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
        measured: id === 2 ? { expPerHour: 1840, activeSeconds: 1260 } : null,
      };
    });
  return { from: from == null ? null : ref(from), spots };
}
