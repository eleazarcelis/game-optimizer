import { containsPath, type Executable } from "./model";

export type DetectionTarget = "steam" | "games" | "all";
export type DetectionGame = { app_id: string; name: string; candidates: string[] };
export type DetectionIssue = { kind: string; path: string | null };
export type DetectionResult = { steam: string | null; games: DetectionGame[]; issues: DetectionIssue[] };
export type DetectedSelection = { path: string; kind: "steam" | "game" };

export function hasSteamEntry(entries: Executable[]): boolean {
  return entries.some((entry) => entry.path.replace(/\//g, "\\").split("\\").pop()?.toLowerCase() === "steam.exe");
}

export function initialGameSelection(game: DetectionGame, entries: Executable[]): string {
  return game.candidates.length === 1 && !containsPath(entries, game.candidates[0]) ? game.candidates[0] : "";
}
