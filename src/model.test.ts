import { describe, expect, it } from "vitest";
import { availableProfileActions, containsPath, defaultGlobalSettings, fileName, initialState, isAppState, isExe, profileChanged } from "./model";
import { dontShowAgainText, profileName, translate } from "./i18n";

describe("library state", () => {
  it("recognizes duplicate Windows paths case-insensitively", () => {
    expect(containsPath([{ path: "C:\\Games\\Test.exe", profileId: "game" }], "c:\\games\\test.EXE")).toBe(true);
    expect(containsPath([], "C:\\Games\\Test.exe")).toBe(false);
  });
  it("accepts exe extensions and extracts Windows names", () => {
    expect(isExe("C:\\Games\\a.EXE")).toBe(true);
    expect(isExe("C:\\Games\\a.exe.txt")).toBe(false);
    expect(fileName("C:\\Games\\a.exe")).toBe("a.exe");
  });
  it("rejects malformed or unsupported persisted state", () => {
    expect(isAppState(initialState())).toBe(true);
    expect(isAppState({ version: 2 })).toBe(false);
    expect(isAppState(null)).toBe(false);
    const invalid = initialState();
    invalid.profiles[0].cpu = "on";
    expect(isAppState(invalid)).toBe(false);
    const missingProfile = initialState();
    missingProfile.executables.push({ path: "C:\\Games\\test.exe", profileId: "missing" });
    expect(isAppState(missingProfile)).toBe(false);
  });
  it("keeps global actions disabled for new and existing states", () => {
    expect(initialState().globalSettings).toEqual({ power: "none", gameMode: "none", gameDvr: "none" });
    const oldState = initialState();
    delete oldState.globalSettings;
    expect(isAppState(oldState)).toBe(true);
    expect(oldState.globalSettings ?? defaultGlobalSettings()).toEqual({ power: "none", gameMode: "none", gameDvr: "none" });
  });
  it("supports older states and validates launch behavior", () => {
    const state = initialState();
    expect(state.launchBehavior).toBe("minimize");
    delete state.launchBehavior;
    expect(isAppState(state)).toBe(true);
    state.launchBehavior = "close";
    expect(isAppState(state)).toBe(true);
    expect(isAppState({ ...state, launchBehavior: "invalid" })).toBe(false);
  });
  it("accepts old states and validates the automatic update preference", () => {
    const state = initialState();
    expect(state.automaticUpdateChecks).toBeUndefined();
    expect(isAppState(state)).toBe(true);
    expect(isAppState({ ...state, automaticUpdateChecks: true })).toBe(true);
    expect(isAppState({ ...state, automaticUpdateChecks: false })).toBe(true);
    expect(isAppState({ ...state, automaticUpdateChecks: "true" })).toBe(false);
  });
  it("accepts old states and validates persisted notice preferences", () => {
    const state = initialState();
    expect(isAppState(state)).toBe(true);
    state.libraryNoticeDismissed = true;
    state.globalNoticeDismissed = true;
    expect(isAppState(state)).toBe(true);
    expect(isAppState({ ...state, libraryNoticeDismissed: "true" })).toBe(false);
    expect(isAppState({ ...state, globalNoticeDismissed: "true" })).toBe(false);
  });
  it("accepts and validates warning preferences on library entries", () => {
    const state = initialState();
    state.executables.push({ path: "C:\\Games\\test.exe", profileId: "game" });
    expect(isAppState(state)).toBe(true);
    state.executables[0].suppressedWarnings = ['["launch","packagedPriorityUnavailable"]'];
    expect(isAppState(JSON.parse(JSON.stringify(state)))).toBe(true);
    expect(isAppState({ ...state, executables: [{ ...state.executables[0], suppressedWarnings: [1] }] })).toBe(false);
    expect(isAppState({ ...state, executables: [{ ...state.executables[0], suppressedWarnings: "yes" }] })).toBe(false);
  });
  it("starts with Defender actions disabled by default", () => {
    expect(initialState().profiles.every((profile) => !profile.defender)).toBe(true);
  });
  it("includes an explicitly enabled Defender action, even without CPU or GPU actions", () => {
    const profile = { ...initialState().profiles[0], defender: true };
    expect(availableProfileActions(profile)).toEqual({
      actions: { cpu: "high", gpu: "high", defender: true },
    });
    expect(availableProfileActions({ ...profile, cpu: "none", gpu: "none" })).toEqual({
      actions: { cpu: "none", gpu: "none", defender: true },
    });
    expect(availableProfileActions({ ...profile, cpu: "none", gpu: "none", defender: false })).toBeNull();
    expect(availableProfileActions(undefined)).toBeNull();
  });
  it("detects only changes to executable profile actions", () => {
    const profile = initialState().profiles[0];
    const applied = { cpu: profile.cpu, gpu: profile.gpu, defender: profile.defender };
    expect(profileChanged(profile, applied)).toBe(false);
    expect(profileChanged({ ...profile, name: "Renamed", power: "none", gameMode: "off" }, applied)).toBe(false);
    expect(profileChanged({ ...profile, cpu: "none" }, applied)).toBe(true);
    expect(profileChanged({ ...profile, gpu: "balanced" }, applied)).toBe(true);
    expect(profileChanged({ ...profile, defender: true }, applied)).toBe(true);
    expect(profileChanged(undefined, applied)).toBe(true);
  });
});

describe("localization", () => {
  it("uses the entry name literally in the warning preference", () => {
    expect(dontShowAgainText("es", "Spotify")).toBe("No mostrar de nuevo para Spotify");
    expect(dontShowAgainText("en", "Spotify")).toBe("Don't show again for Spotify");
    expect(dontShowAgainText("es", "App $&")).toBe("No mostrar de nuevo para App $&");
  });
  it("translates built-in profiles and preserves custom profile names", () => {
    const state = initialState();
    expect(profileName(state.profiles[0], "es")).toBe("Juego");
    expect(profileName({ ...state.profiles[0], builtIn: undefined, name: "My profile" }, "es")).toBe("My profile");
    expect(translate("en", "notOptimized")).not.toBe(translate("es", "notOptimized"));
  });
});
