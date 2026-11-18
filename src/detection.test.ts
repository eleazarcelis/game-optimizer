import { describe, expect, it } from "vitest";
import { hasSteamEntry, initialGameSelection } from "./detection";

describe("Steam detection choices", () => {
  it("recognizes a Steam executable entry without matching other Steam processes", () => {
    expect(hasSteamEntry([{ path: "C:\\Steam\\steam.exe", profileId: "launcher" }])).toBe(true);
    expect(hasSteamEntry([{ path: "C:/Steam/STEAM.EXE", profileId: "launcher" }])).toBe(true);
    expect(hasSteamEntry([{ path: "C:\\Steam\\steamwebhelper.exe", profileId: "launcher" }])).toBe(false);
  });
  it("preselects only a unique executable that is not already in the library", () => {
    const game = { app_id: "1", name: "Game", candidates: ["C:\\Game\\game.exe"] };
    expect(initialGameSelection(game, [])).toBe("C:\\Game\\game.exe");
    expect(initialGameSelection(game, [{ path: "c:\\game\\GAME.exe", profileId: "game" }])).toBe("");
    expect(initialGameSelection(game, [{ path: "c:/game/GAME.exe", profileId: "game" }])).toBe("");
    expect(initialGameSelection({ ...game, candidates: [] }, [])).toBe("");
    expect(initialGameSelection({ ...game, candidates: [...game.candidates, "C:\\Game\\setup.exe"] }, [])).toBe("");
  });
});
