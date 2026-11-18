import type { AppState, LaunchBehavior } from "./model";

export type EntryWarning = { path: string; id: string };
export type WarningNotification = { message: string; entry?: EntryWarning & { name: string } };

export function warningId(kind: "launch" | "apply" | "restore", ...details: (string | boolean)[]): string {
  return JSON.stringify([kind, ...details]);
}

export function warningSuppressed(state: AppState | undefined, warning: EntryWarning): boolean {
  return !!state?.executables.find((item) => item.path.toLowerCase() === warning.path.toLowerCase())
    ?.suppressedWarnings?.includes(warning.id);
}

export function setEntryWarnings(state: AppState, path: string, ids: string[]): AppState {
  return {
    ...state,
    executables: state.executables.map((item) => {
      if (item.path.toLowerCase() !== path.toLowerCase()) return item;
      return { ...item, suppressedWarnings: ids };
    }),
  };
}

export function launchWindowAction(behavior: LaunchBehavior, warningShown: boolean): "close" | "minimize" | "stay" {
  return behavior === "close" ? "close" : warningShown ? "stay" : "minimize";
}
