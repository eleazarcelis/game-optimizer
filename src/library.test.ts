import { describe, expect, it } from "vitest";
import { moveExecutables, organizeLibrary, planLibraryMove } from "./library";
import { initialState, isAppState, type Executable } from "./model";

const item = (name: string, favorite = false): Executable => ({ path: `C:\\Games\\${name}.exe`, profileId: "game", favorite });

describe("library ordering", () => {
  it("sorts displayed names in either direction with favorites first without changing manual order", () => {
    const items = [item("Zulu"), item("Alpha"), item("Beta", true)];
    const names = { [items[0].path]: "Apple" };
    const order = (sort: "az" | "za" | "manual") => organizeLibrary(items, names, sort, false, "en").map((group) => group.entries[0].item.path);
    expect(order("az")).toEqual([items[2].path, items[1].path, items[0].path]);
    expect(order("za")).toEqual([items[2].path, items[0].path, items[1].path]);
    expect(order("manual")).toEqual([items[2].path, items[0].path, items[1].path]);
    expect(items[0].path).toContain("Zulu");
  });
  it("groups related names and retains ordered group and member positions", () => {
    const items = [item("NTE"), item("Steam"), item("NTELauncher"), item("Steam WebHelper"), item("Other")];
    const grouped = organizeLibrary(items, {}, "manual", true, "en");
    expect(grouped.map((group) => group.entries.map(({ item: member }) => member.path))).toEqual([
      [items[0].path, items[2].path], [items[1].path, items[3].path], [items[4].path],
    ]);
    items[3].favorite = true;
    const favoriteGroups = organizeLibrary(items, {}, "az", true, "en");
    expect(favoriteGroups[0].entries[0].item.path).toBe(items[3].path);
    expect(organizeLibrary(items, {}, "manual", false, "en")).toHaveLength(5);
  });
  it("uses the chosen name order for groups and their members", () => {
    const items = [item("NTE"), item("Steam"), item("NTELauncher"), item("Steam WebHelper")];
    const groups = organizeLibrary(items, {}, "za", true, "en");
    expect(groups.map((group) => group.name)).toEqual(["Steam", "NTE"]);
    expect(groups[1].entries.map(({ item: member }) => member.path)).toEqual([items[2].path, items[0].path]);
  });
  it("moves items and entire groups without losing unrelated entries", () => {
    const items = [item("A"), item("B"), item("C"), item("D")];
    expect(moveExecutables(items, [items[0].path, items[2].path], [items[3].path], true)).toEqual([items[1], items[3], items[0], items[2]]);
    expect(moveExecutables(items, [items[2].path], [items[0].path], false)).toEqual([items[2], items[0], items[1], items[3]]);
    expect(moveExecutables(items, [items[0].path], [items[0].path], false)).toEqual(items);
  });
  it("only plans a move when the requested visible insertion actually changes the order", () => {
    const items = [item("A"), item("B"), item("C"), item("Fav", true)];
    const plan = (source: number, target: number, after: boolean) => planLibraryMove(items, {}, false, "en", null, [items[source].path], [items[target].path], after);
    expect(plan(0, 0, true)).toBeNull();
    expect(plan(0, 1, false)).toBeNull();
    expect(plan(0, 1, true)?.map((entry) => entry.path)).toEqual([items[1].path, items[0].path, items[2].path, items[3].path]);
    expect(plan(2, 3, false)).toBeNull();
  });
  it("checks group and member positions before accepting a grouped move", () => {
    const items = [item("NTE"), item("Steam"), item("NTELauncher"), item("Steam WebHelper")];
    const nte = [items[0].path, items[2].path];
    const steam = [items[1].path, items[3].path];
    const movedGroup = planLibraryMove(items, {}, true, "en", null, steam, nte, false);
    expect(movedGroup && organizeLibrary(movedGroup, {}, "manual", true, "en").map((group) => group.name)).toEqual(["Steam", "NTE"]);
    const movedMember = planLibraryMove(items, {}, true, "en", "nte", [items[2].path], [items[0].path], false);
    expect(movedMember && organizeLibrary(movedMember, {}, "manual", true, "en")[0].entries.map(({ item: member }) => member.path)).toEqual([items[2].path, items[0].path]);
    expect(planLibraryMove(items, {}, true, "en", "nte", [items[0].path], [items[1].path], true)).toBeNull();
    items[2].favorite = true;
    expect(planLibraryMove(items, {}, true, "en", "nte", [items[0].path], [items[2].path], false)).toBeNull();
  });
  it("accepts existing state and validates new persisted preferences", () => {
    const state = initialState();
    expect(isAppState(state)).toBe(true);
    expect(isAppState({ ...state, librarySort: "za", groupByName: true, executables: [item("Test", true)] })).toBe(true);
    expect(isAppState({ ...state, librarySort: "other" })).toBe(false);
    expect(isAppState({ ...state, groupByName: "yes" })).toBe(false);
    expect(isAppState({ ...state, executables: [{ ...item("Test"), favorite: "yes" }] })).toBe(false);
  });
});
