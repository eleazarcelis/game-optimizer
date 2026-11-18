import { describe, expect, it } from "vitest";
import { launchWindowAction, setEntryWarnings, warningId, warningSuppressed } from "./entry-warnings";
import { initialState } from "./model";

describe("entry warning preferences", () => {
  it("isolates each warning by entry and kind, persists, and can be reset", () => {
    const state = initialState();
    state.executables = [
      { path: "C:\\Apps\\one.exe", profileId: "game" },
      { path: "C:\\Apps\\two.exe", profileId: "game" },
    ];
    const warning = { path: "c:\\apps\\ONE.exe", id: warningId("launch", "packagedPriorityUnavailable") };
    const otherKind = { ...warning, id: warningId("launch", "jobUnavailable") };
    expect(warningSuppressed(state, warning)).toBe(false);
    const updated = setEntryWarnings(state, warning.path, [warning.id]);
    const reloaded = JSON.parse(JSON.stringify(updated));
    expect(warningSuppressed(reloaded, warning)).toBe(true);
    expect(warningSuppressed(reloaded, otherKind)).toBe(false);
    expect(warningSuppressed(reloaded, { ...warning, path: "C:\\Apps\\two.exe" })).toBe(false);
    expect(warningSuppressed(setEntryWarnings(reloaded, warning.path, []), warning)).toBe(false);
    expect(warningSuppressed(state, warning)).toBe(false);
  });

  it("does not suppress a different operation or failure", () => {
    const first = warningId("apply", "defender-path", "overlap", false);
    expect(first).not.toBe(warningId("restore", "defender-path", "overlap", false));
    expect(first).not.toBe(warningId("apply", "defender-path", "access denied", false));
    expect(first).not.toBe(warningId("apply", "defender-path", "overlap", true));
  });

  it("always honors close after launch, even when a warning is shown", () => {
    expect(launchWindowAction("close", true)).toBe("close");
    expect(launchWindowAction("close", false)).toBe("close");
    expect(launchWindowAction("minimize", true)).toBe("stay");
    expect(launchWindowAction("minimize", false)).toBe("minimize");
  });
});
