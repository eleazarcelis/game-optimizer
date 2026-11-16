import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { Gamepad2, Globe2, Layers3, Settings2 } from "lucide-react";
import type { Language } from "../model";
import { translate as t } from "../i18n";

type Props = { page: "library" | "settings" | "global"; setPage: (page: "library" | "settings" | "global") => void; language: Language; showError: (error: unknown) => void };
export function Sidebar({ page, setPage, language, showError }: Props) {
  const [version, setVersion] = useState<string>();
  useEffect(() => {
    let active = true;
    getVersion().then((value) => { if (active) setVersion(value); }).catch((error: unknown) => { if (active) showError(error); });
    return () => { active = false; };
  }, [showError]);
  return <aside className="sidebar">
    <div className="brand"><div className="brand-icon"><Gamepad2 /></div><div><div className="brand-name"><strong>GAME<span>OPT</span></strong>{version && <span className="app-version" title={`v${version}`}>v{version}</span>}</div><small>{t(language, "controlCenter")}</small></div></div>
    <p className="side-label">{t(language, "workspace")}</p>
    <button className={`nav-item ${page === "library" ? "active" : ""}`} onClick={() => setPage("library")}><Layers3 size={19} />{t(language, "library")}</button>
    <button className={`nav-item ${page === "global" ? "active" : ""}`} onClick={() => setPage("global")}><Globe2 size={19} />{t(language, "global")}</button>
    <button className={`nav-item ${page === "settings" ? "active" : ""}`} onClick={() => setPage("settings")}><Settings2 size={19} />{t(language, "settings")}</button>
  </aside>;
}
