import { afterEach, describe, expect, it, vi } from "vitest";
import { checkForUpdates, compareVersions } from "./updates";

const release = (tag: string, assets: unknown[] = []) => ({
  tag_name: tag, draft: false, prerelease: false, assets,
});
const asset = (tag: string) => ({ name: `Game-Optimizer-${tag}-windows-x64.exe`, state: "uploaded" });

afterEach(() => vi.unstubAllGlobals());

describe("update checks", () => {
  it("compares numeric and prerelease versions using semantic version precedence", () => {
    expect(compareVersions("2.10.0", "2.9.9")).toBeGreaterThan(0);
    expect(compareVersions("2.1.1", "2.1.1-rc.2")).toBeGreaterThan(0);
    expect(compareVersions("2.1.1-rc.10", "2.1.1-rc.2")).toBeGreaterThan(0);
    expect(compareVersions("2.1.1-rc.1", "2.1.1-rc.1.1")).toBeLessThan(0);
    expect(compareVersions("2.1.1-1", "2.1.1-alpha")).toBeLessThan(0);
    expect(compareVersions("2.1.1", "2.1.1")).toBe(0);
    expect(() => compareVersions("2.1.1-rc.01", "2.1.1")).toThrow();
  });

  it("offers only the matching uploaded Windows executable for a newer release", async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => release("v2.2.0", [asset("v2.2.0")]) });
    vi.stubGlobal("fetch", fetchMock);
    expect(await checkForUpdates("2.1.1")).toEqual({ tag: "v2.2.0", downloadable: true });
    expect(fetchMock.mock.calls[0][0]).toBe("https://api.github.com/repos/eleazarcelis/game-optimizer/releases/latest");
    expect(fetchMock.mock.calls[0][1].credentials).toBe("omit");
  });

  it("reports a newer release without an uploaded executable and ignores older releases", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: true, json: async () => release("v2.2.0", [{ ...asset("v2.2.0"), state: "new" }, asset("v2.3.0")]) }));
    expect(await checkForUpdates("2.1.1")).toEqual({ tag: "v2.2.0", downloadable: false });
    expect(await checkForUpdates("2.2.0")).toBeNull();
    expect(await checkForUpdates("2.3.0")).toBeNull();
  });

  it("surfaces API errors and rejects malformed release data", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: false, status: 403 }));
    await expect(checkForUpdates("2.1.1")).rejects.toThrow("GitHub HTTP 403");
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: true, json: async () => release("v2.2.0/evil") }));
    await expect(checkForUpdates("2.1.1")).rejects.toThrow("Invalid version");
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: true, json: async () => ({ ...release("v2.2.0"), prerelease: true }) }));
    await expect(checkForUpdates("2.1.1")).rejects.toThrow("Invalid GitHub release response");
  });
});
