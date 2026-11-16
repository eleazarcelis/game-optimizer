import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { Gamepad2 } from "lucide-react";
import type { AppState, Language, LaunchBehavior } from "../model";
import { translate as t } from "../i18n";
import { Profiles } from "./Profiles";
import type { UpdateStatus } from "../updates";

type Props = { state: AppState; update: (state: AppState) => void; showError: (message: string) => void; initialTab?: SettingsTab; updateStatus: UpdateStatus; checkUpdates: () => void };
type SettingsTab = "general" | "profiles" | "about";

export function Settings({ state, update, showError, initialTab = "general", updateStatus, checkUpdates }: Props) {
  const lang = state.language;
  const [tab, setTab] = useState<SettingsTab>(initialTab);
  const [version, setVersion] = useState<string>();
  const generalRef = useRef<HTMLButtonElement>(null);
  const profilesRef = useRef<HTMLButtonElement>(null);
  const aboutRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    let active = true;
    getVersion().then((value) => { if (active) setVersion(value); }).catch((error: unknown) => {
      if (active) showError(String(error));
    });
    return () => { active = false; };
  }, [showError]);
  function navigateTabs(event: KeyboardEvent<HTMLDivElement>) {
    let next: SettingsTab;
    if (event.key === "Home") next = "general";
    else if (event.key === "End") next = "about";
    else if (event.key === "ArrowLeft") next = tab === "general" ? "about" : tab === "profiles" ? "general" : "profiles";
    else if (event.key === "ArrowRight") next = tab === "general" ? "profiles" : tab === "profiles" ? "about" : "general";
    else return;
    event.preventDefault();
    setTab(next);
    (next === "general" ? generalRef : next === "profiles" ? profilesRef : aboutRef).current?.focus();
  }
  return <section className="page">
    <header className="page-header"><div><span className="eyebrow">GAME OPTIMIZER / {t(lang, "settings").toUpperCase()}</span><h1>{t(lang, "settings")}</h1><p>{t(lang, "settingsSubtitle")}</p></div></header>
    <div className="settings-tabs" role="tablist" aria-label={t(lang, "settings")} onKeyDown={navigateTabs}>
      <button ref={generalRef} id="tab-general" role="tab" aria-selected={tab === "general"} aria-controls="panel-general" tabIndex={tab === "general" ? 0 : -1} onClick={() => setTab("general")}>{t(lang, "general")}</button>
      <button ref={profilesRef} id="tab-profiles" role="tab" aria-selected={tab === "profiles"} aria-controls="panel-profiles" tabIndex={tab === "profiles" ? 0 : -1} onClick={() => setTab("profiles")}>{t(lang, "profiles")}</button>
      <button ref={aboutRef} id="tab-about" role="tab" aria-selected={tab === "about"} aria-controls="panel-about" tabIndex={tab === "about" ? 0 : -1} onClick={() => setTab("about")}>{t(lang, "about")}</button>
    </div>
    <div id="panel-general" className="settings-panel" role="tabpanel" aria-labelledby="tab-general" hidden={tab !== "general"}><div className="profile-editor settings-general scroll-panel" role="region" aria-label={t(lang, "general")} tabIndex={0}>
      <label className="field"><span>{t(lang, "language")}</span><select value={lang} onChange={(e) => update({ ...state, language: e.target.value as Language })}><option value="en">English</option><option value="es">Español</option></select></label>
      <label className="field"><span>{t(lang, "launchBehavior")}<small className="field-description">{t(lang, "launchBehaviorDesc")}</small></span><select value={state.launchBehavior ?? "minimize"} onChange={(e) => update({ ...state, launchBehavior: e.target.value as LaunchBehavior })}><option value="minimize">{t(lang, "minimizeApp")}</option><option value="close">{t(lang, "closeApp")}</option></select></label>
    </div></div>
    <div id="panel-profiles" className="settings-panel" role="tabpanel" aria-labelledby="tab-profiles" hidden={tab !== "profiles"}><Profiles state={state} update={update} showError={showError} /></div>
    <div id="panel-about" className="settings-panel" role="tabpanel" aria-labelledby="tab-about" hidden={tab !== "about"}>
      <div className="settings-about profile-editor scroll-panel" role="region" aria-label={t(lang, "about")} tabIndex={0}>
        <div className="about-heading"><div className="brand-icon"><Gamepad2 /></div><div><h2>Game Optimizer</h2><p>{t(lang, "aboutDescription")}</p></div></div>
        <div className="about-version"><span>{t(lang, "version")}</span><strong>{version ? `v${version}` : t(lang, "loading")}</strong></div>
        <label className="field checkbox"><span>{t(lang, "automaticUpdates")}<small className="field-description">{t(lang, "automaticUpdatesDesc")}</small></span><input type="checkbox" checked={state.automaticUpdateChecks === true} onChange={(e) => update({ ...state, automaticUpdateChecks: e.target.checked })}/></label>
        <div className="update-actions"><button className="secondary" disabled={updateStatus.kind === "checking"} onClick={checkUpdates}>{t(lang, updateStatus.kind === "checking" ? "checkingUpdates" : "checkUpdates")}</button>
          {updateStatus.kind === "available" && updateStatus.downloadable && <button className="primary" onClick={() => { void invoke("open_update_download", { tag: updateStatus.tag }).catch((error: unknown) => showError(`${t(lang, "updateDownloadFailed")} ${String(error && typeof error === "object" && "message" in error ? error.message : error)}`)); }}>{t(lang, "downloadUpdate")}</button>}
        </div>
        <div className="update-status" role="status" aria-live="polite">
          {updateStatus.kind === "current" && t(lang, "updateCurrent")}
          {updateStatus.kind === "failed" && t(lang, "updateCheckFailed")}
          {updateStatus.kind === "available" && <><strong>{t(lang, "updateAvailable")} {updateStatus.tag}</strong><p>{t(lang, updateStatus.downloadable ? "updateDownloadDesc" : "updateAssetMissing")}</p></>}
        </div>
      </div>
    </div>
  </section>;
}
