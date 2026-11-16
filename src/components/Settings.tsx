import { useRef, useState, type KeyboardEvent } from "react";
import type { AppState, Language, LaunchBehavior } from "../model";
import { translate as t } from "../i18n";
import { Profiles } from "./Profiles";

type Props = { state: AppState; update: (state: AppState) => void; showError: (message: string) => void };

export function Settings({ state, update, showError }: Props) {
  const lang = state.language;
  const [tab, setTab] = useState<"general" | "profiles">("general");
  const generalRef = useRef<HTMLButtonElement>(null);
  const profilesRef = useRef<HTMLButtonElement>(null);
  function navigateTabs(event: KeyboardEvent<HTMLDivElement>) {
    let next: "general" | "profiles";
    if (event.key === "Home") next = "general";
    else if (event.key === "End") next = "profiles";
    else if (event.key === "ArrowLeft" || event.key === "ArrowRight") next = tab === "general" ? "profiles" : "general";
    else return;
    event.preventDefault();
    setTab(next);
    (next === "general" ? generalRef : profilesRef).current?.focus();
  }
  return <section className="page">
    <header className="page-header"><div><span className="eyebrow">GAME OPTIMIZER / {t(lang, "settings").toUpperCase()}</span><h1>{t(lang, "settings")}</h1><p>{t(lang, "settingsSubtitle")}</p></div></header>
    <div className="settings-tabs" role="tablist" aria-label={t(lang, "settings")} onKeyDown={navigateTabs}>
      <button ref={generalRef} id="tab-general" role="tab" aria-selected={tab === "general"} aria-controls="panel-general" tabIndex={tab === "general" ? 0 : -1} onClick={() => setTab("general")}>{t(lang, "general")}</button>
      <button ref={profilesRef} id="tab-profiles" role="tab" aria-selected={tab === "profiles"} aria-controls="panel-profiles" tabIndex={tab === "profiles" ? 0 : -1} onClick={() => setTab("profiles")}>{t(lang, "profiles")}</button>
    </div>
    <div id="panel-general" className="settings-panel" role="tabpanel" aria-labelledby="tab-general" hidden={tab !== "general"}><div className="profile-editor settings-general scroll-panel" role="region" aria-label={t(lang, "general")} tabIndex={0}>
      <label className="field"><span>{t(lang, "language")}</span><select value={lang} onChange={(e) => update({ ...state, language: e.target.value as Language })}><option value="en">English</option><option value="es">Español</option></select></label>
      <label className="field"><span>{t(lang, "launchBehavior")}<small className="field-description">{t(lang, "launchBehaviorDesc")}</small></span><select value={state.launchBehavior ?? "minimize"} onChange={(e) => update({ ...state, launchBehavior: e.target.value as LaunchBehavior })}><option value="minimize">{t(lang, "minimizeApp")}</option><option value="close">{t(lang, "closeApp")}</option></select></label>
    </div></div>
    <div id="panel-profiles" className="settings-panel" role="tabpanel" aria-labelledby="tab-profiles" hidden={tab !== "profiles"}><Profiles state={state} update={update} showError={showError} /></div>
  </section>;
}
