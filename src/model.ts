export type Language = "en" | "es";
export type Choice = "none" | "low" | "high" | "balanced" | "on" | "off";
export type Profile = {
  id: string;
  name: string;
  builtIn?: "game" | "launcher" | "secondary";
  cpu: Choice;
  gpu: Choice;
  power: Choice;
  gameMode: Choice;
  gameDvr: Choice;
  defender: boolean;
};
export type ProfileActions = Pick<Profile, "cpu" | "gpu" | "defender">;
export type Executable = { path: string; profileId: string; favorite?: boolean; suppressedWarnings?: string[] };
export type LibrarySort = "az" | "za" | "manual";
export type GlobalSettings = { power: Choice; gameMode: Choice; gameDvr: Choice };
export type LaunchBehavior = "minimize" | "close";
export type AppState = {
  version: 1;
  language: Language;
  view: "grid" | "list";
  iconSize: number;
  librarySort?: LibrarySort;
  groupByName?: boolean;
  profiles: Profile[];
  executables: Executable[];
  globalSettings?: GlobalSettings;
  launchBehavior?: LaunchBehavior;
  libraryNoticeDismissed?: boolean;
  globalNoticeDismissed?: boolean;
  automaticUpdateChecks?: boolean;
};

export function defaultGlobalSettings(): GlobalSettings {
  return { power: "none", gameMode: "none", gameDvr: "none" };
}

export function initialState(): AppState {
  return {
    version: 1, language: "en", view: "grid", iconSize: 96, executables: [], globalSettings: defaultGlobalSettings(), launchBehavior: "minimize",
    profiles: [
      { id: "game", name: "Game", builtIn: "game", cpu: "high", gpu: "high", power: "high", gameMode: "on", gameDvr: "off", defender: false },
      { id: "launcher", name: "Launcher", builtIn: "launcher", cpu: "low", gpu: "balanced", power: "balanced", gameMode: "on", gameDvr: "none", defender: false },
      { id: "secondary", name: "Secondary process", builtIn: "secondary", cpu: "none", gpu: "balanced", power: "balanced", gameMode: "off", gameDvr: "none", defender: false },
    ],
  };
}

export function containsPath(items: Executable[], path: string): boolean {
  return items.some((item) => item.path.toLocaleLowerCase("en-US") === path.toLocaleLowerCase("en-US"));
}

export function isExe(path: string): boolean {
  return path.toLowerCase().endsWith(".exe");
}

export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

export function availableProfileActions(profile: Profile | undefined) {
  if (!profile || (profile.cpu === "none" && profile.gpu === "none" && !profile.defender)) return null;
  return {
    actions: { cpu: profile.cpu, gpu: profile.gpu, defender: profile.defender },
  };
}

export function profileChanged(profile: Profile | undefined, applied: ProfileActions): boolean {
  return !profile || profile.cpu !== applied.cpu || profile.gpu !== applied.gpu || profile.defender !== applied.defender;
}

export function isAppState(value: unknown): value is AppState {
  if (!value || typeof value !== "object") return false;
  const state = value as Partial<AppState>;
  if (state.version !== 1 || (state.language !== "en" && state.language !== "es")
    || (state.view !== "grid" && state.view !== "list") || typeof state.iconSize !== "number"
    || !Number.isInteger(state.iconSize) || state.iconSize < 40 || state.iconSize > 96
    || !Array.isArray(state.profiles) || state.profiles.length === 0 || !Array.isArray(state.executables)) return false;
  const validProfiles = state.profiles.every((p) => p && typeof p.id === "string" && p.id.length > 0
    && typeof p.name === "string" && ["game", "launcher", "secondary", undefined].includes(p.builtIn)
    && ["none", "low", "high"].includes(p.cpu) && ["none", "balanced", "high"].includes(p.gpu)
    && ["none", "balanced", "high"].includes(p.power) && ["none", "on", "off"].includes(p.gameMode)
    && ["none", "on", "off"].includes(p.gameDvr) && typeof p.defender === "boolean");
  if (!validProfiles || new Set(state.profiles.map((p) => p.id)).size !== state.profiles.length) return false;
  if (state.globalSettings !== undefined && (!state.globalSettings
    || !["none", "balanced", "high"].includes(state.globalSettings.power)
    || !["none", "on", "off"].includes(state.globalSettings.gameMode)
    || !["none", "on", "off"].includes(state.globalSettings.gameDvr))) return false;
  if (state.launchBehavior !== undefined && !["minimize", "close"].includes(state.launchBehavior)) return false;
  if (state.libraryNoticeDismissed !== undefined && typeof state.libraryNoticeDismissed !== "boolean") return false;
  if (state.globalNoticeDismissed !== undefined && typeof state.globalNoticeDismissed !== "boolean") return false;
  if (state.librarySort !== undefined && !["az", "za", "manual"].includes(state.librarySort)) return false;
  if (state.groupByName !== undefined && typeof state.groupByName !== "boolean") return false;
  if (state.automaticUpdateChecks !== undefined && typeof state.automaticUpdateChecks !== "boolean") return false;
  const ids = new Set(state.profiles.map((p) => p.id));
  return state.executables.every((e) => e && typeof e.path === "string" && isExe(e.path)
    && typeof e.profileId === "string" && ids.has(e.profileId)
    && (e.favorite === undefined || typeof e.favorite === "boolean")
    && (e.suppressedWarnings === undefined || (Array.isArray(e.suppressedWarnings) && e.suppressedWarnings.every((warning) => typeof warning === "string"))))
    && new Set(state.executables.map((e) => e.path.toLowerCase())).size === state.executables.length;
}
