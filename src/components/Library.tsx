import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FolderOpen, Gamepad2, Grid2X2, List, LoaderCircle, MoreVertical, Play, Plus, RotateCcw, Trash2, X, Zap } from "lucide-react";
import type { AppState, Executable, Language } from "../model";
import { availableProfileActions, fileName } from "../model";
import { profileName, translate as t } from "../i18n";
import { DismissibleNotice } from "./DismissibleNotice";

type OperationProps = { activeOperations: string[]; claimOperation: (path: string) => boolean; releaseOperation: (path: string) => void };
type Props = OperationProps & { state: AppState; update: (state: AppState) => void; add: () => void; launch: (path: string) => void; launchingPath: string | null; confirmAction: (message: string) => Promise<boolean>; ensureSaved: () => Promise<void>; showError: (error: unknown) => void };
type GpuStatus = { state: "none" | "applied" | "defaults" | "recovery" | "conflict" | "unverified" | "resetCommit"; detail: string | null };
function ExecutableIcon({ path, language, launch, disabled }: { path: string; language: Language; launch: () => void; disabled: boolean }) {
  const [icon, setIcon] = useState<string>();
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let active = true;
    invoke<string>("inspect_executable", { path }).then((value) => { if (active) setIcon(value); }).catch(() => { if (active) setFailed(true); });
    return () => { active = false; };
  }, [path]);
  return <button className="app-icon" disabled={disabled} onClick={launch} title={failed ? `ICON_READ: ${t(language, "iconError")}` : t(language, "launch")} aria-label={`${t(language, "launch")}: ${fileName(path)}`}>{icon ? <img src={icon} alt="" /> : <Gamepad2 size={48} />}</button>;
}
function Entry({ item, state, remove, launch, launchingPath, confirmAction, ensureSaved, showError, activeOperations, claimOperation, releaseOperation }: OperationProps & { item: Executable; state: AppState; remove: () => void; launch: (path: string) => void; launchingPath: string | null; confirmAction: (message: string) => Promise<boolean>; ensureSaved: () => Promise<void>; showError: (error: unknown) => void }) {
  const lang = state.language;
  const profile = state.profiles.find((p) => p.id === item.profileId);
  const [status, setStatus] = useState<GpuStatus>();
  const [name, setName] = useState<string>();
  const [nameFailed, setNameFailed] = useState(false);
  const busy = activeOperations.includes(item.path.toLowerCase());
  const launchDisabled = activeOperations.length > 0 || launchingPath !== null;
  const [statusFailed, setStatusFailed] = useState(false);
  const [statusErrorDismissed, setStatusErrorDismissed] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const menu = useRef<HTMLDivElement>(null);
  const menuButton = useRef<HTMLButtonElement>(null);
  const latestRemove = useRef(remove);
  latestRemove.current = remove;
  useEffect(() => {
    if (!menuOpen) return;
    function dismiss(event: PointerEvent) {
      if (event.target instanceof Node && !menu.current?.contains(event.target)) setMenuOpen(false);
    }
    function escape(event: KeyboardEvent) {
      if (event.key === "Escape") { setMenuOpen(false); menuButton.current?.focus(); }
    }
    document.addEventListener("pointerdown", dismiss);
    document.addEventListener("keydown", escape);
    return () => { document.removeEventListener("pointerdown", dismiss); document.removeEventListener("keydown", escape); };
  }, [menuOpen]);
  useEffect(() => {
    let active = true;
    setName(undefined);
    setNameFailed(false);
    invoke<string | null>("executable_name", { path: item.path })
      .then((value) => { if (active) setName(value ?? undefined); })
      .catch(() => { if (active) setNameFailed(true); });
    return () => { active = false; };
  }, [item.path]);
  useEffect(() => {
    if (busy) return;
    let active = true;
    setStatus(undefined);
    setStatusFailed(false);
    setStatusErrorDismissed(false);
    invoke<GpuStatus>("gpu_status", { path: item.path })
      .then((result) => { if (active) setStatus(result); })
      .catch((error: unknown) => { if (active) { setStatusFailed(true); showError(error); } });
    return () => { active = false; };
  }, [item.path, showError, busy]);
  const plan = availableProfileActions(profile);
  const resetPlan = profile && { actions: { cpu: profile.cpu, gpu: profile.gpu, defender: false } };
  async function perform(action: "apply" | "restore" | "reset") {
    let claimed = false;
    try {
      const selectedPlan = action === "reset" ? resetPlan : plan;
      const details = selectedPlan && (action === "apply" || action === "reset")
        ? `\n\n${t(lang, "cpu")}: ${t(lang, selectedPlan.actions.cpu)}\n${t(lang, "gpu")}: ${t(lang, selectedPlan.actions.gpu === "balanced" ? "gpuSaving" : selectedPlan.actions.gpu)}\n${t(lang, "defender")}: ${t(lang, action === "reset" ? "none" : selectedPlan.actions.defender ? "on" : "off")}${action === "apply" && selectedPlan.actions.defender ? `\n${t(lang, "defenderWarning")}` : ""}`
        : "";
      if (!await confirmAction(t(lang, action === "apply" ? "gpuConfirm" : action === "reset" ? "resetConfirm" : status?.state === "resetCommit" ? "finishResetConfirm" : "restoreConfirm") + details)) return;
      if (!claimOperation(item.path)) return;
      claimed = true;
      await ensureSaved();
      if (action !== "restore") {
        if (!selectedPlan) throw new Error(t(lang, "profileUnsupported"));
        await invoke(action === "apply" ? "apply_profile" : "reset_profile", { path: item.path, profile: selectedPlan.actions });
      } else {
        await invoke("restore_profile", { path: item.path });
      }
    } catch (error) {
      showError(error);
    } finally {
      if (claimed) {
        try {
          setStatus(await invoke<GpuStatus>("gpu_status", { path: item.path }));
          setStatusFailed(false);
        } catch (error) {
          setStatus(undefined);
          setStatusFailed(true);
          setStatusErrorDismissed(false);
          showError(error);
        }
        releaseOperation(item.path);
      }
    }
  }
  const statusLabel = statusFailed ? t(lang, statusErrorDismissed ? "notOptimized" : "statusError") : !status ? t(lang, "loading")
    : t(lang, status.state === "none" ? "gpuNone" : status.state === "applied" ? "gpuApplied" : status.state === "defaults" ? "defaultsApplied" : status.state === "conflict" ? "gpuConflict" : status.state === "unverified" ? "defenderStatusUnverified" : status.state === "resetCommit" ? "resetCommitStatus" : "gpuRecovery");
  const canReset = status && ["none", "applied", "defaults"].includes(status.state);
  const hasResetActions = !!profile && (profile.cpu !== "none" || profile.gpu !== "none" || status?.state === "applied" || status?.state === "defaults");
  async function confirmRemove() {
    setMenuOpen(false);
    menuButton.current?.focus();
    if (!await confirmAction(`${t(lang, "removeConfirm")}\n\n${name ?? fileName(item.path)}`)) return;
    if (!claimOperation(item.path)) return;
    try {
      const current = await invoke<GpuStatus>("gpu_status", { path: item.path });
      if (current.state !== "none") {
        setStatus(current);
        showError({ code: "RECOVERY_REQUIRED", message: t(lang, "removeBlocked") });
        return;
      }
      latestRemove.current();
    } catch (error) {
      showError(error);
    } finally {
      releaseOperation(item.path);
    }
  }
  return <article className="entry">
    <div className="entry-header">
      <ExecutableIcon path={item.path} language={lang} launch={() => launch(item.path)} disabled={launchDisabled} />
      <div className="entry-content">
        <div className="entry-name">
          <h3 title={nameFailed ? `NAME_READ: ${t(lang, "nameError")}` : name ?? fileName(item.path)}>{name ?? fileName(item.path)}</h3>
          <button className="icon-button entry-launch" disabled={launchDisabled} title={launchDisabled ? t(lang, "launchBusy") : t(lang, "launch")} aria-label={t(lang, "launch")} onClick={() => launch(item.path)}>{launchingPath === item.path ? <LoaderCircle className="spin" size={16}/> : <Play size={16}/>}</button>
          <div className="entry-menu" ref={menu} onBlur={(event) => { if (!event.currentTarget.contains(event.relatedTarget)) setMenuOpen(false); }}>
            <button ref={menuButton} className="icon-button" title={t(lang, "moreActions")} aria-label={t(lang, "moreActions")} aria-expanded={menuOpen} onClick={() => setMenuOpen((open) => !open)}><MoreVertical size={18}/></button>
            {menuOpen && <div className="entry-menu-panel">
              <button onClick={() => { setMenuOpen(false); void invoke("open_executable_folder", { path: item.path }).catch(showError); }}><FolderOpen size={16}/>{t(lang, "openFolder")}</button>
              <button className="entry-menu-danger" disabled={busy || launchingPath !== null || status?.state !== "none"} title={status?.state === "none" ? t(lang, "remove") : t(lang, "removeBlocked")} onClick={() => void confirmRemove()}><Trash2 size={16}/>{t(lang, "remove")}</button>
            </div>}
          </div>
        </div>
        <div className="entry-location"><span className="path" title={item.path}>{item.path}</span></div>
      </div>
    </div>
    <div className="entry-meta"><span className="profile-tag">{profile ? profileName(profile, lang) : t(lang, "noProfile")}</span><span className="status" title={status?.detail ?? undefined}><i />{busy ? <><LoaderCircle className="spin" size={12}/>{t(lang, "working")}</> : statusLabel}</span>{statusFailed && !statusErrorDismissed && <button className="icon-button status-dismiss" title={t(lang, "dismissStatusError")} aria-label={t(lang, "dismissStatusError")} onClick={() => setStatusErrorDismissed(true)}><X size={14}/></button>}</div>
    <div className="entry-actions entry-profile-actions">
      {status && status.state !== "none"
        ? <button className="entry-restore" disabled={busy || launchingPath !== null} onClick={() => void perform("restore")}><RotateCcw size={14} aria-hidden="true"/>{t(lang, status.state === "resetCommit" ? "finishReset" : "revert")}</button>
        : <button className="entry-optimize" disabled={busy || launchingPath !== null || status?.state !== "none" || !plan} title={!plan ? t(lang, "profileUnsupported") : undefined} onClick={() => void perform("apply")}><Zap size={14} aria-hidden="true"/>{t(lang, "optimize")}</button>}
      <button disabled={busy || launchingPath !== null || !canReset || !hasResetActions} title={!canReset ? t(lang, "resetBlocked") : !hasResetActions ? t(lang, "resetUnsupported") : undefined} onClick={() => void perform("reset")}>{t(lang, "reset")}</button>
    </div>
  </article>;
}
export function Library({ state, update, confirmAction, add, launch, launchingPath, ensureSaved, showError, activeOperations, claimOperation, releaseOperation }: Props) {
  const lang: Language = state.language;
  return <section><header className="page-header"><div><span className="eyebrow">GAME OPTIMIZER / {t(lang, "library").toUpperCase()}</span><h1>{t(lang, "title")}</h1><p>{t(lang, "subtitle")}</p></div><button className="primary" onClick={add}><Plus size={18}/>{t(lang, "add")}</button></header>
    {!state.libraryNoticeDismissed && <DismissibleNotice language={lang} dismiss={() => update({ ...state, libraryNoticeDismissed: true })}>{t(lang, "gpuPreview")}</DismissibleNotice>}
    <div className="toolbar"><span className="count">{state.executables.length} {t(lang, "library").toLowerCase()}</span><div className="toolbar-actions"><div className="segmented"><button className={state.view === "grid" ? "selected" : ""} title={t(lang, "grid")} aria-label={t(lang, "grid")} onClick={() => update({ ...state, view: "grid" })}><Grid2X2 size={17}/></button><button className={state.view === "list" ? "selected" : ""} title={t(lang, "list")} aria-label={t(lang, "list")} onClick={() => update({ ...state, view: "list" })}><List size={17}/></button></div></div></div>
    {state.executables.length === 0 ? <div className="empty"><div className="empty-icon"><Gamepad2 size={32}/></div><h2>{t(lang, "empty")}</h2><p>{t(lang, "emptyText")}</p><button className="secondary" onClick={add}><Plus size={16}/>{t(lang, "add")}</button></div> : <div className={state.view === "grid" ? "entry-grid" : "entry-list"}>{state.executables.map((item) => <Entry key={item.path.toLowerCase()} item={item} state={state} launch={launch} launchingPath={launchingPath} confirmAction={confirmAction} ensureSaved={ensureSaved} showError={showError} activeOperations={activeOperations} claimOperation={claimOperation} releaseOperation={releaseOperation} remove={() => update({ ...state, executables: state.executables.filter((e) => e.path !== item.path) })} />)}</div>}
  </section>;
}
