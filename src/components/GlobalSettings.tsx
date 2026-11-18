import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LoaderCircle, RotateCcw, X, Zap } from "lucide-react";
import { defaultGlobalSettings, type AppState, type Choice, type GlobalSettings as Settings } from "../model";
import { translate as t } from "../i18n";
import { skippedText, type SkippedStep } from "../errors";
import { DismissibleNotice } from "./DismissibleNotice";

type Props = { state: AppState; update: (state: AppState) => void; confirmAction: (message: string) => Promise<boolean>; ensureSaved: () => Promise<void>; showError: (error: unknown) => void; activeOperations: string[]; launching: boolean; claimOperation: (key: string) => boolean; releaseOperation: (key: string) => void };
type Status = { state: "none" | "applied" | "recovery" | "conflict"; detail: string | null };
const fields: { key: keyof Settings; values: Choice[] }[] = [
  { key: "power", values: ["none", "balanced", "high"] },
  { key: "gameMode", values: ["none", "on", "off"] },
  { key: "gameDvr", values: ["none", "on", "off"] },
];

export function GlobalSettings({ state, update, confirmAction, ensureSaved, showError, activeOperations, launching, claimOperation, releaseOperation }: Props) {
  const lang = state.language;
  const settings = state.globalSettings ?? defaultGlobalSettings();
  const [status, setStatus] = useState<Status>();
  const [failed, setFailed] = useState(false);
  const [statusErrorDismissed, setStatusErrorDismissed] = useState(false);
  const busy = activeOperations.includes("global-settings");
  useEffect(() => {
    if (busy) return;
    let active = true;
    setStatus(undefined);
    setFailed(false);
    setStatusErrorDismissed(false);
    invoke<Status>("global_status")
      .then((result) => { if (active) setStatus(result); })
      .catch((error: unknown) => { if (active) { setFailed(true); showError(error); } });
    return () => { active = false; };
  }, [busy, showError]);
  const supported = settings.power !== "none" || settings.gameMode !== "none" || settings.gameDvr !== "none";
  async function perform(action: "apply" | "restore") {
    let claimed = false;
    try {
      if (!await confirmAction(t(lang, action === "apply" ? "globalConfirm" : "globalRestoreConfirm"))) return;
      if (!claimOperation("global-settings")) return;
      claimed = true;
      await ensureSaved();
      if (action === "apply") await invoke("apply_global", { settings });
      else {
        const skipped = await invoke<SkippedStep[]>("restore_global");
        skipped.forEach((step) => showError({ warning: skippedText(lang, [step]) }));
      }
    } catch (error) {
      showError(error);
    } finally {
      if (claimed) releaseOperation("global-settings");
    }
  }
  const statusLabel = failed ? t(lang, statusErrorDismissed ? "notOptimized" : "globalStatusError") : !status ? t(lang, "loading")
    : t(lang, status.state === "none" ? "globalNone" : status.state === "applied" ? "globalApplied" : status.state === "conflict" ? "globalConflict" : "gpuRecovery");
  return <section className="page">
    <header className="page-header"><div><span className="eyebrow">GAME OPTIMIZER / {t(lang, "global").toUpperCase()}</span><h1>{t(lang, "global")}</h1><p>{t(lang, "globalSubtitle")}</p></div></header>
    {!state.globalNoticeDismissed && <DismissibleNotice language={lang} dismiss={() => update({ ...state, globalNoticeDismissed: true })}>{t(lang, "globalNote")} {t(lang, "globalPreview")}</DismissibleNotice>}
    <div className="profile-editor global-editor scroll-panel" role="region" aria-label={t(lang, "global")} tabIndex={0}>
      {fields.map(({ key, values }) => <label className="field" key={key}>
        <span>{t(lang, key)}<small className="field-description">{t(lang, key === "power" ? "powerDesc" : key === "gameMode" ? "gameModeDesc" : "gameDvrDesc")}</small></span>
        <select value={settings[key]} onChange={(e) => update({ ...state, globalSettings: { ...settings, [key]: e.target.value as Choice } })}>
          {values.map((value) => <option key={value} value={value}>{t(lang, key === "power" && value === "balanced" ? "powerBalanced" : value)}</option>)}
        </select>
      </label>)}
      <div className="global-status"><p className="status" role="status" title={status?.detail ?? undefined}>{busy ? <><LoaderCircle className="spin" size={12}/>{t(lang, "working")}</> : statusLabel}</p>{failed && !statusErrorDismissed && <button className="icon-button status-dismiss" title={t(lang, "dismissStatusError")} aria-label={t(lang, "dismissStatusError")} onClick={() => setStatusErrorDismissed(true)}><X size={14}/></button>}</div>
      <div className="entry-actions"><button className="entry-optimize" disabled={busy || launching || status?.state !== "none" || !supported} title={!supported ? t(lang, "globalUnsupported") : undefined} onClick={() => void perform("apply")}><Zap size={14} aria-hidden="true"/>{t(lang, "optimize")}</button><button className="entry-restore" disabled={busy || launching || !status || status.state === "none"} onClick={() => void perform("restore")}><RotateCcw size={14} aria-hidden="true"/>{t(lang, "revert")}</button></div>
    </div>
  </section>;
}
