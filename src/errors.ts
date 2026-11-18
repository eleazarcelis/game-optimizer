import { translate as t, type Key } from "./i18n";
import type { Language } from "./model";

export type SkippedStep = { step: string; message: string; retryable: boolean };
export type ApplyWarning = {
  step: string; message: string; recovery_required: boolean;
  folder_conflict?: { target: string; folders: string[]; unidentified: boolean };
};

const codeKeys: Record<string, Key> = {
  NO_BACKUP: "errNoBackup", NO_ACTION: "errNoAction", RECOVERY_REQUIRED: "errRecoveryRequired", ACTION_CONFLICT: "errConflict",
  JOURNAL_CONFLICT: "errConflict", JOURNAL_INVALID: "errJournal", JOURNAL_LOCKED: "errLocked", JOURNAL_LOCK: "errLocked",
  INVALID_PROFILE: "errInvalidProfile", INVALID_SETTINGS: "errInvalidProfile", INVALID_EXECUTABLE: "errInvalidExecutable",
  RESTORE_FAILED: "errRestoreFailed", APPLY_FAILED: "errApplyFailed", SNAPSHOT_FAILED: "errApplyFailed", SNAPSHOT_INVALID: "errApplyFailed",
  DATA_TOO_LARGE: "errDataInvalid", DATA_INVALID: "errDataInvalid", LAUNCH_FAILED: "errLaunch", FOLDER_OPEN: "errFolder",
  RESET_INVALID: "errJournal", RESET_FINALIZE: "errRestoreFailed", ACTION_CHECK: "errRestoreFailed", UNKNOWN: "errUnknown",
};
const stepKeys: Record<string, Key> = {
  cpu: "cpu", gpu: "gpu", power: "power", gameMode: "gameMode", gameDvr: "gameDvr", "defender-path": "defenderFolder", "defender-process": "defenderProcess",
};

export function stepName(language: Language, step: string): string {
  const key = stepKeys[step];
  return key ? t(language, key) : step;
}

export function skippedText(language: Language, skipped: SkippedStep[]): string {
  return skipped.map((s) => `${stepName(language, s.step)}: ${t(language, s.retryable ? "warnRestoreRetry" : "warnSkippedExternal")}`).join("\n");
}

export function applyWarningText(language: Language, warning: ApplyWarning): string {
  if (warning.step === "defender-path" && warning.message === "Defender exclusion overlaps another active profile") {
    const conflict = warning.folder_conflict;
    if (!conflict) return `${t(language, "warnFolderSkipped")} ${t(language, "errDefenderOverlap")}`;
    const describe = (path: string) => `${path.split(/[\\/]/).filter(Boolean).pop() ?? path} (${path})`;
    const lines = [t(language, "warnFolderSkipped"), `${t(language, "warnFolder")}: ${describe(conflict.target)}`];
    for (const folder of conflict.folders) lines.push(`${t(language, "warnFolderConflict")}: ${describe(folder)}`);
    if (conflict.unidentified) lines.push(t(language, "warnFolderUnknown"));
    return lines.join("\n");
  }
  return `${t(language, "warnApplySkipped")}\n${localizeError(language, "APPLY_FAILED", `${warning.step}: ${warning.message}`)}`;
}

export function localizeError(language: Language, code: string, message: string): string {
  if (["APPLY_FAILED", "SNAPSHOT_FAILED", "SNAPSHOT_INVALID", "LAUNCH_FAILED"].includes(code)) {
    const split = message.indexOf(": ");
    const step = split < 0 ? "" : message.slice(0, split);
    const detail = step in stepKeys ? message.slice(split + 2) : message;
    const cause = detail.split("; recovery required")[0].split(/; previous (state|settings) restored/)[0];
    const reason: Key | undefined =
      cause.includes("Defender exclusion overlaps another active profile") ? "errDefenderOverlap" :
      cause.includes("Defender reservation directory has unsafe permissions or ownership") ? "errDefenderUnsafe" :
      /Defender (resource or reservation|resource|reservation) identity changed/.test(cause) ? "errDefenderIdentity" :
      cause.includes("Defender reservation is missing") ? "errDefenderMissing" :
      cause.includes("Defender exclusion changed since snapshot") ? "errDefenderChanged" :
      cause.includes("Defender exclusion state requires an elevated administrator token") ? "errAdminRequired" :
      cause.includes("Defender process exclusion belongs to another active profile") ? "errDefenderProcessOverlap" :
      cause.includes("Cannot identify a unique packaged application") ? "errPackageAmbiguous" :
      /os error 5\b/i.test(cause) ? "errAccessDenied" : undefined;
    const parts = [`${code}: ${t(language, codeKeys[code])}`];
    if (step in stepKeys) parts.push(`${stepName(language, step)}:`);
    if (reason) parts.push(t(language, reason));
    if (/; previous (state|settings) restored/.test(detail)) parts.push(t(language, "errPreviousRestored"));
    if (detail.includes("; recovery required")) parts.push(t(language, "errRecoveryNeeded"));
    if (!reason || detail.includes("; recovery required:")) {
      parts.push(`${t(language, "errTechnicalDetails")}: ${detail}`);
    }
    return parts.join(" ");
  }
  if (message.includes("changed externally")) {
    const step = message.includes(": ") ? message.slice(0, message.indexOf(": ")) : "";
    return `${code}: ${step in stepKeys ? `${stepName(language, step)}: ` : ""}${t(language, "errConflict")}`;
  }
  const key = codeKeys[code];
  return `${code}: ${t(language, key ?? "errUnknown")}`;
}
