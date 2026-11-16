import { useEffect, useRef, useState } from "react";
import { Plus, ShieldAlert, X } from "lucide-react";
import type { AppState, Choice, Profile } from "../model";
import { profileName, translate as t } from "../i18n";

type Props = { state: AppState; update: (state: AppState) => void; showError: (message: string) => void };
type Action = "cpu" | "gpu";
const fields: { key: Action; values: Choice[] }[] = [
  { key: "cpu", values: ["none", "low", "high"] }, { key: "gpu", values: ["none", "balanced", "high"] },
];
export function Profiles({ state, update, showError }: Props) {
  const lang = state.language;
  const [selected, setSelected] = useState(state.profiles[0]?.id);
  const [dismissedWarnings, setDismissedWarnings] = useState<string[]>([]);
  const list = useRef<HTMLDivElement>(null);
  useEffect(() => {
    list.current?.querySelector(".profile-selected")?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [selected]);
  const profile = state.profiles.find((p) => p.id === selected);
  const change = (next: Profile) => update({ ...state, profiles: state.profiles.map((p) => p.id === next.id ? next : p) });
  const add = () => {
    const item: Profile = { id: crypto.randomUUID(), name: lang === "es" ? "Perfil personalizado" : "Custom profile", cpu: "none", gpu: "none", power: "none", gameMode: "none", gameDvr: "none", defender: false };
    update({ ...state, profiles: [...state.profiles, item] });
    setSelected(item.id);
  };
  const remove = () => {
    if (!profile || profile.builtIn) return;
    if (state.executables.some((item) => item.profileId === profile.id)) { showError(t(lang, "inUse")); return; }
    update({ ...state, profiles: state.profiles.filter((p) => p.id !== profile.id) });
    setSelected(state.profiles[0]?.id);
  };
  return <section className="page"><header className="page-header profile-heading"><div><h2>{t(lang, "profileTitle")}</h2><p>{t(lang, "profileSubtitle")}</p></div><button className="primary" onClick={add}><Plus size={18}/>{t(lang, "create")}</button></header>
    <div className="profile-layout"><div ref={list} className="profile-list scroll-panel" role="region" aria-label={t(lang, "profiles")} tabIndex={0}>{state.profiles.map((p) => <button key={p.id} className={p.id === selected ? "profile-selected" : ""} onClick={() => setSelected(p.id)}>{profileName(p, lang)}<span>›</span></button>)}</div>
      {profile && <div className="profile-editor scroll-panel" role="region" aria-label={profileName(profile, lang)} tabIndex={0}><h2>{profileName(profile, lang)}</h2>{!profile.builtIn && <label className="field">{t(lang, "profileName")}<input value={profile.name} maxLength={60} onChange={(e) => change({ ...profile, name: e.target.value })}/></label>}
        {fields.map(({ key, values }) => <label className="field" key={key}><span>{t(lang, key)}<small className="field-description">{t(lang, key === "cpu" ? "cpuDesc" : "gpuDesc")}</small></span><select value={profile[key]} onChange={(e) => change({ ...profile, [key]: e.target.value as Choice })}>{values.map((v) => <option value={v} key={v}>{t(lang, key === "gpu" && v === "balanced" ? "gpuSaving" : v)}</option>)}</select></label>)}
        <label className="field checkbox"><span>{t(lang, "defender")}<small className="field-description">{t(lang, "defenderDesc")}</small></span><input type="checkbox" checked={profile.defender} onChange={(e) => change({ ...profile, defender: e.target.checked })}/></label>
        {profile.defender && !dismissedWarnings.includes(profile.id) && <div className="warning"><ShieldAlert size={18}/><span>{t(lang, "defenderWarning")}</span><button className="icon-button" title={t(lang, "dismissNotice")} aria-label={t(lang, "dismissNotice")} onClick={() => setDismissedWarnings([...dismissedWarnings, profile.id])}><X size={16}/></button></div>}
        {!profile.builtIn && <button className="danger" onClick={remove}>{t(lang, "delete")}</button>}
      </div>}</div>
  </section>;
}
