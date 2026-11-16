import { fileName, type Executable, type Language, type LibrarySort } from "./model";

export type LibraryEntry = { item: Executable; name: string };
export type LibraryGroup = { key: string; name: string; entries: LibraryEntry[] };

function stem(name: string): string {
  return name.replace(/\.exe$/i, "").trim();
}

function normalize(name: string): string {
  return stem(name).toLocaleLowerCase("en-US");
}

export function organizeLibrary(items: Executable[], names: Record<string, string>, sort: LibrarySort, grouped: boolean, language: Language): LibraryGroup[] {
  const entries = items.map((item) => ({ item, name: names[item.path] || fileName(item.path) }));
  const collator = new Intl.Collator(language, { sensitivity: "base", numeric: true });
  if (sort !== "manual") entries.sort((a, b) => (sort === "az" ? 1 : -1) * collator.compare(a.name, b.name));
  entries.sort((a, b) => Number(!!b.item.favorite) - Number(!!a.item.favorite));
  if (!grouped) return entries.map((entry) => ({ key: entry.item.path, name: entry.name, entries: [entry] }));

  const candidates = [...new Set(entries.flatMap(({ item, name }) => [stem(name), stem(fileName(item.path))]).filter((name) => normalize(name).length >= 3))]
    .sort((a, b) => normalize(a).length - normalize(b).length || collator.compare(a, b));
  const assigned = new Set<string>();
  const groups: LibraryGroup[] = [];
  for (const candidate of candidates) {
    const prefix = normalize(candidate);
    const matches = entries.filter(({ item, name }) => !assigned.has(item.path)
      && [name, fileName(item.path)].some((value) => normalize(value).startsWith(prefix)));
    if (matches.length < 2) continue;
    for (const { item } of matches) assigned.add(item.path);
    groups.push({ key: prefix, name: candidate, entries: matches });
  }
  for (const entry of entries) {
    if (!assigned.has(entry.item.path)) groups.push({ key: entry.item.path, name: entry.name, entries: [entry] });
  }
  const index = new Map(entries.map(({ item }, i) => [item.path, i]));
  return groups.sort((a, b) => (index.get(a.entries[0].item.path) ?? 0) - (index.get(b.entries[0].item.path) ?? 0));
}

export function moveExecutables(items: Executable[], movingPaths: string[], targetPaths: string[], after: boolean): Executable[] {
  const moving = new Set(movingPaths);
  const target = new Set(targetPaths);
  const selected = items.filter((item) => moving.has(item.path));
  const remaining = items.filter((item) => !moving.has(item.path));
  if (!selected.length || movingPaths.some((path) => target.has(path))) return items;
  const indices = remaining.map((item, index) => target.has(item.path) ? index : -1).filter((index) => index >= 0);
  if (!indices.length) return items;
  const position = after ? indices[indices.length - 1] + 1 : indices[0];
  remaining.splice(position, 0, ...selected);
  return remaining;
}

export function planLibraryMove(items: Executable[], names: Record<string, string>, grouped: boolean, language: Language, groupKey: string | null, movingPaths: string[], targetPaths: string[], after: boolean): Executable[] | null {
  const groups = organizeLibrary(items, names, "manual", grouped, language);
  const paths = (group: LibraryGroup) => group.entries.map(({ item }) => item.path);
  const peers = groupKey === null ? groups.map(paths) : (groups.find((group) => group.key === groupKey)?.entries.map(({ item }) => [item.path]) ?? []);
  const matches = (peer: string[], selected: string[]) => peer.length === selected.length && peer.every((path) => selected.includes(path));
  const sourceIndex = peers.findIndex((peer) => matches(peer, movingPaths));
  const targetIndex = peers.findIndex((peer) => matches(peer, targetPaths));
  if (sourceIndex < 0 || targetIndex < 0 || sourceIndex === targetIndex) return null;

  const expected = [...peers];
  const [moving] = expected.splice(sourceIndex, 1);
  const insertAt = expected.findIndex((peer) => matches(peer, targetPaths)) + Number(after);
  expected.splice(insertAt, 0, moving);
  if (expected.every((peer, index) => matches(peer, peers[index]))) return null;

  const reordered = moveExecutables(items, movingPaths, targetPaths, after);
  const result = organizeLibrary(reordered, names, "manual", grouped, language);
  if (groupKey !== null && (result.length !== groups.length || result.some((group, index) => group.key !== groups[index].key))) return null;
  const actual = groupKey === null ? result.map(paths) : (result.find((group) => group.key === groupKey)?.entries.map(({ item }) => [item.path]) ?? []);
  if (actual.length !== expected.length || actual.some((peer, index) => !matches(peer, expected[index]))) return null;
  if (groupKey === null && result.some((group) => {
    const previous = groups.find((candidate) => candidate.key === group.key);
    return !previous || paths(group).some((path, index) => path !== paths(previous)[index]);
  })) return null;
  return reordered;
}
