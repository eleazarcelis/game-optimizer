import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { CheckCircle2, ExternalLink, Gamepad2 } from "lucide-react";
import type { AppState, Language, LaunchBehavior } from "../model";
import { translate as t } from "../i18n";
import { Profiles } from "./Profiles";
import type { UpdateStatus } from "../updates";
import { hasSteamEntry, type DetectionTarget } from "../detection";
import licenseText from "../../LICENSE?raw";

type Props = { state: AppState; update: (state: AppState) => void; showError: (message: string) => void; initialTab?: SettingsTab; updateStatus: UpdateStatus; checkUpdates: () => void; detect: (target: DetectionTarget) => void; detecting: boolean; detectedSteam: string | null; detectedGameCount: number | null };
type SettingsTab = "general" | "profiles" | "detection" | "about";

export function Settings({ state, update, showError, initialTab = "general", updateStatus, checkUpdates, detect, detecting, detectedSteam, detectedGameCount }: Props) {
  const lang = state.language;
  const steamAdded = hasSteamEntry(state.executables);
  const [tab, setTab] = useState<SettingsTab>(initialTab);
  const [version, setVersion] = useState<string>();
  const generalRef = useRef<HTMLButtonElement>(null);
  const profilesRef = useRef<HTMLButtonElement>(null);
  const detectionRef = useRef<HTMLButtonElement>(null);
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
    else if (event.key === "ArrowLeft") next = tab === "general" ? "about" : tab === "profiles" ? "general" : tab === "detection" ? "profiles" : "detection";
    else if (event.key === "ArrowRight") next = tab === "general" ? "profiles" : tab === "profiles" ? "detection" : tab === "detection" ? "about" : "general";
    else return;
    event.preventDefault();
    setTab(next);
    (next === "general" ? generalRef : next === "profiles" ? profilesRef : next === "detection" ? detectionRef : aboutRef).current?.focus();
  }
  return <section className="page">
    <header className="page-header"><div><span className="eyebrow">GAME OPTIMIZER / {t(lang, "settings").toUpperCase()}</span><h1>{t(lang, "settings")}</h1><p>{t(lang, "settingsSubtitle")}</p></div></header>
    <div className="settings-tabs" role="tablist" aria-label={t(lang, "settings")} onKeyDown={navigateTabs}>
      <button ref={generalRef} id="tab-general" role="tab" aria-selected={tab === "general"} aria-controls="panel-general" tabIndex={tab === "general" ? 0 : -1} onClick={() => setTab("general")}>{t(lang, "general")}</button>
      <button ref={profilesRef} id="tab-profiles" role="tab" aria-selected={tab === "profiles"} aria-controls="panel-profiles" tabIndex={tab === "profiles" ? 0 : -1} onClick={() => setTab("profiles")}>{t(lang, "profiles")}</button>
      <button ref={detectionRef} id="tab-detection" role="tab" aria-selected={tab === "detection"} aria-controls="panel-detection" tabIndex={tab === "detection" ? 0 : -1} onClick={() => setTab("detection")}>{t(lang, "automaticDetection")}</button>
      <button ref={aboutRef} id="tab-about" role="tab" aria-selected={tab === "about"} aria-controls="panel-about" tabIndex={tab === "about" ? 0 : -1} onClick={() => setTab("about")}>{t(lang, "about")}</button>
    </div>
    <div id="panel-general" className="settings-panel" role="tabpanel" aria-labelledby="tab-general" hidden={tab !== "general"}><div className="profile-editor settings-general scroll-panel" role="region" aria-label={t(lang, "general")} tabIndex={0}>
      <label className="field"><span>{t(lang, "language")}</span><select value={lang} onChange={(e) => update({ ...state, language: e.target.value as Language })}><option value="en">English</option><option value="es">Español</option></select></label>
      <label className="field"><span>{t(lang, "launchBehavior")}<small className="field-description">{t(lang, "launchBehaviorDesc")}</small></span><select value={state.launchBehavior ?? "minimize"} onChange={(e) => update({ ...state, launchBehavior: e.target.value as LaunchBehavior })}><option value="minimize">{t(lang, "minimizeApp")}</option><option value="close">{t(lang, "closeApp")}</option></select></label>
    </div></div>
    <div id="panel-profiles" className="settings-panel" role="tabpanel" aria-labelledby="tab-profiles" hidden={tab !== "profiles"}><Profiles state={state} update={update} showError={showError} /></div>
    <div id="panel-detection" className="settings-panel" role="tabpanel" aria-labelledby="tab-detection" hidden={tab !== "detection"}><div className="profile-editor settings-general scroll-panel" role="region" aria-label={t(lang, "automaticDetection")} tabIndex={0}>
      <p className="field-description">{t(lang, "detectDescription")}</p>
      <div className="detection-actions">
        <div className="detection-action">
          <div className="detection-icon"><svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M11.979 0C5.678 0 .511 4.86.022 11.037l6.432 2.658c.545-.371 1.203-.59 1.912-.59.063 0 .125.004.188.006l2.861-4.142V8.91c0-2.495 2.028-4.524 4.524-4.524 2.494 0 4.524 2.031 4.524 4.527s-2.03 4.525-4.524 4.525h-.105l-4.076 2.911c0 .052.004.105.004.159 0 1.875-1.515 3.396-3.39 3.396-1.635 0-3.016-1.173-3.331-2.727L.436 15.27C1.862 20.307 6.486 24 11.979 24c6.627 0 11.999-5.373 11.999-12S18.605 0 11.979 0zM7.54 18.21l-1.473-.61c.262.543.714.999 1.314 1.25 1.297.539 2.793-.076 3.332-1.375.263-.63.264-1.319.005-1.949s-.75-1.121-1.377-1.383c-.624-.26-1.29-.249-1.878-.03l1.523.63c.956.4 1.409 1.5 1.009 2.455-.397.957-1.497 1.41-2.454 1.012H7.54zm11.415-9.303c0-1.662-1.353-3.015-3.015-3.015-1.665 0-3.015 1.353-3.015 3.015 0 1.665 1.35 3.015 3.015 3.015 1.663 0 3.015-1.35 3.015-3.015zm-5.273-.005c0-1.252 1.013-2.266 2.265-2.266 1.249 0 2.266 1.014 2.266 2.266 0 1.251-1.017 2.265-2.266 2.265-1.253 0-2.265-1.014-2.265-2.265z"/></svg></div>
          <div className="detection-info"><strong>Steam</strong>{steamAdded && <span className="detection-added"><CheckCircle2 size={15} aria-hidden="true"/>{t(lang, "detectAdded")}</span>}</div>
          <button className="secondary" disabled={detecting} onClick={() => detect("steam")} aria-label={t(lang, "detectSteam")}>{t(lang, "detectSearch")}</button>
        </div>
        <div className="detection-action">
          <div className="detection-icon"><Gamepad2 size={22} aria-hidden="true"/></div>
          <div className="detection-info"><strong>{t(lang, "steamGames")}</strong>{detectedGameCount !== null && <span className="detection-count" role="status">{t(lang, "detectGameCount")}: {detectedGameCount}</span>}</div>
          <button className="secondary" disabled={detecting || (!detectedSteam && !steamAdded)} onClick={() => detect("games")} aria-label={t(lang, "detectSteamGames")}>{t(lang, "detectSearch")}</button>
        </div>
        {!detectedSteam && !steamAdded && <p className="field-description">{t(lang, "detectSteamFirst")}</p>}
      </div>
      {detecting && <p className="field-description" role="status">{t(lang, "detecting")}</p>}
    </div></div>
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
        <div className="about-feedback">
          <button className="secondary" onClick={() => { void invoke("open_issue_report").catch((error: unknown) => showError(`${t(lang, "issueOpenFailed")} ${String(error && typeof error === "object" && "message" in error ? error.message : error)}`)); }}><ExternalLink size={16} aria-hidden="true"/>{t(lang, "reportIssue")}</button>
          <p>{t(lang, "reportIssueDesc")}</p>
        </div>
        <div className="about-license">
          <strong>{t(lang, "license")}</strong>
          <p>GNU General Public License v3.0 {t(lang, "licenseOrLater")} (GPL-3.0-or-later)</p>
          <p>{t(lang, "sourceCode")}: github.com/eleazarcelis/game-optimizer</p>
          <details><summary>{t(lang, "viewLicense")}</summary><pre className="license-text scroll-panel" tabIndex={0}>{licenseText}</pre></details>
        </div>
      </div>
    </div>
  </section>;
}
