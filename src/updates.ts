const RELEASES_API = "https://api.github.com/repos/eleazarcelis/game-optimizer/releases/latest";
const VERSION_PATTERN = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?$/;

type Version = { numbers: bigint[]; prerelease: string[] | null };
export type UpdateResult = { tag: string; downloadable: boolean } | null;
export type UpdateStatus =
  | { kind: "idle" | "checking" | "current" | "failed" }
  | { kind: "available"; tag: string; downloadable: boolean };

function parseVersion(value: string): Version {
  const match = VERSION_PATTERN.exec(value);
  if (!match || value.length > 200) throw new Error(`Invalid version: ${value}`);
  return { numbers: [BigInt(match[1]), BigInt(match[2]), BigInt(match[3])], prerelease: match[4]?.split(".") ?? null };
}

function compareNumbers(a: bigint, b: bigint): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

export function compareVersions(a: string, b: string): number {
  const left = parseVersion(a);
  const right = parseVersion(b);
  for (let i = 0; i < 3; i++) {
    const result = compareNumbers(left.numbers[i], right.numbers[i]);
    if (result) return result;
  }
  if (!left.prerelease || !right.prerelease) return left.prerelease ? -1 : right.prerelease ? 1 : 0;
  for (let i = 0; i < Math.min(left.prerelease.length, right.prerelease.length); i++) {
    const x = left.prerelease[i], y = right.prerelease[i];
    if (x === y) continue;
    const xNumeric = /^\d+$/.test(x), yNumeric = /^\d+$/.test(y);
    if (xNumeric && yNumeric) return compareNumbers(BigInt(x), BigInt(y));
    if (xNumeric !== yNumeric) return xNumeric ? -1 : 1;
    return x < y ? -1 : 1;
  }
  return compareNumbers(BigInt(left.prerelease.length), BigInt(right.prerelease.length));
}

export async function checkForUpdates(currentVersion: string): Promise<UpdateResult> {
  parseVersion(currentVersion);
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 15000);
  try {
    const response = await fetch(RELEASES_API, {
      headers: { Accept: "application/vnd.github+json" },
      credentials: "omit",
      cache: "no-store",
      signal: controller.signal,
    });
    if (!response.ok) throw new Error(`GitHub HTTP ${response.status}`);
    const data: unknown = await response.json();
    if (!data || typeof data !== "object" || !("tag_name" in data) || typeof data.tag_name !== "string"
      || !data.tag_name.startsWith("v") || !("assets" in data) || !Array.isArray(data.assets)
      || !("draft" in data) || data.draft !== false || !("prerelease" in data) || data.prerelease !== false) {
      throw new Error("Invalid GitHub release response");
    }
    const tag = data.tag_name;
    if (compareVersions(tag.slice(1), currentVersion) <= 0) return null;
    const assetName = `Game-Optimizer-${tag}-windows-x64.exe`;
    const downloadable = data.assets.some((asset: unknown) => asset && typeof asset === "object"
      && "name" in asset && asset.name === assetName && "state" in asset && asset.state === "uploaded");
    return { tag, downloadable };
  } finally {
    clearTimeout(timeout);
  }
}
