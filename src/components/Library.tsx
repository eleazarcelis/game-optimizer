import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChevronDown, ChevronRight, FolderOpen, Gamepad2, GripVertical, LoaderCircle, MoreVertical, Play, Plus, RotateCcw, Search, Star, Trash2, X, Zap } from "lucide-react";
import type { AppState, Executable, Language, LibrarySort, ProfileActions } from "../model";
import { availableProfileActions, fileName, profileChanged } from "../model";
import { organizeLibrary, planLibraryMove, type LibraryEntry, type LibraryGroup } from "../library";
import { profileName, translate as t } from "../i18n";
import { applyWarningText, skippedText, localizeError, type ApplyWarning, type SkippedStep } from "../errors";
import { DismissibleNotice } from "./DismissibleNotice";
import { warningId } from "../entry-warnings";

type OperationProps = { activeOperations: string[]; claimOperation: (path: string) => boolean; releaseOperation: (path: string) => void };
type Props = OperationProps & { state: AppState; update: (state: AppState) => void; add: () => void; detect: () => void; detecting: boolean; launch: (path: string, name: string) => void; launchingPath: string | null; confirmAction: (message: string) => Promise<boolean>; ensureSaved: () => Promise<void>; showError: (error: unknown) => void; showEntryWarning: (path: string, name: string, id: string, message: string) => void; resetEntryWarnings: (path: string) => void; warningSaving: boolean };
type GpuStatus = { state: "none" | "applied" | "partial" | "defaults" | "recovery" | "conflict" | "unverified" | "resetCommit"; detail: string | null; appliedProfile: ProfileActions | null };
type DragHandlers = Pick<React.HTMLAttributes<HTMLElement>, "onPointerDown" | "onPointerMove" | "onPointerUp" | "onPointerCancel" | "onLostPointerCapture">;
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
function Entry({ item, name, nameFailed, state, remove, favorite, dragHandle, dragHandlers, dropScope, launch, launchingPath, confirmAction, ensureSaved, showError, showEntryWarning, resetEntryWarnings, warningSaving, activeOperations, claimOperation, releaseOperation }: OperationProps & { item: Executable; name?: string; nameFailed: boolean; state: AppState; remove: () => void; favorite: () => void; dragHandle?: React.ReactNode; dragHandlers?: DragHandlers; dropScope?: string; launch: Props["launch"]; launchingPath: string | null; confirmAction: (message: string) => Promise<boolean>; ensureSaved: () => Promise<void>; showError: (error: unknown) => void; showEntryWarning: Props["showEntryWarning"]; resetEntryWarnings: Props["resetEntryWarnings"]; warningSaving: boolean }) {
  const lang = state.language;
  const entryName = name ?? fileName(item.path);
  const profile = state.profiles.find((p) => p.id === item.profileId);
  const [status, setStatus] = useState<GpuStatus>();
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
    menu.current?.querySelector(".entry-menu-panel")?.scrollIntoView({ block: "nearest", inline: "nearest" });
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
  async function perform(action: "apply" | "restore" | "reset" | "reapply") {
    const warnSkipped = (skipped: SkippedStep[]) => { skipped.forEach((step) => showEntryWarning(item.path, entryName, warningId("restore", step.step, step.message, step.retryable), skippedText(lang, [step]))); };
    const warnApply = (warnings: ApplyWarning[]) => { warnings.forEach((step) => showEntryWarning(item.path, entryName, warningId("apply", step.step, step.message, step.recovery_required), applyWarningText(lang, step))); };
    let claimed = false;
    let restoredForReapply = false;
    try {
      const selectedPlan = action === "reset" ? resetPlan : plan;
      const details = selectedPlan && (action === "apply" || action === "reset" || action === "reapply")
        ? `\n\n${t(lang, "cpu")}: ${t(lang, selectedPlan.actions.cpu)}\n${t(lang, "gpu")}: ${t(lang, selectedPlan.actions.gpu === "balanced" ? "gpuSaving" : selectedPlan.actions.gpu)}\n${t(lang, "defender")}: ${t(lang, action === "reset" ? "none" : selectedPlan.actions.defender ? "on" : "off")}${action !== "reset" && selectedPlan.actions.defender ? `\n${t(lang, "defenderWarning")}` : ""}`
        : "";
      if (!await confirmAction(t(lang, action === "apply" ? "gpuConfirm" : action === "reapply" ? "reapplyConfirm" : action === "reset" ? "resetConfirm" : status?.state === "resetCommit" ? "finishResetConfirm" : "restoreConfirm") + details)) return;
      if (!claimOperation(item.path)) return;
      claimed = true;
      await ensureSaved();
      if (action === "reapply") {
        if (!selectedPlan) throw new Error(t(lang, "profileUnsupported"));
        const skipped = await invoke<SkippedStep[]>("restore_profile", { path: item.path });
        warnSkipped(skipped);
        if (skipped.length) return;
        restoredForReapply = true;
        warnApply(await invoke<ApplyWarning[]>("apply_profile", { path: item.path, profile: selectedPlan.actions }));
      } else if (action !== "restore") {
        if (!selectedPlan) throw new Error(t(lang, "profileUnsupported"));
        if (action === "apply") warnApply(await invoke<ApplyWarning[]>("apply_profile", { path: item.path, profile: selectedPlan.actions }));
        else await invoke("reset_profile", { path: item.path, profile: selectedPlan.actions });
      } else {
        warnSkipped(await invoke<SkippedStep[]>("restore_profile", { path: item.path }));
      }
    } catch (error) {
      if (restoredForReapply) {
        const detail = error && typeof error === "object" && "code" in error ? localizeError(lang, String(error.code), String((error as { message?: unknown }).message ?? "")) : String(error && typeof error === "object" && "message" in error ? error.message : error);
        showError({ code: "REAPPLY_FAILED", message: `${t(lang, "reapplyFailed")} ${detail}` });
      } else showError(error);
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
    : t(lang, status.state === "none" ? "gpuNone" : status.state === "applied" ? "gpuApplied" : status.state === "partial" ? "gpuPartial" : status.state === "defaults" ? "defaultsApplied" : status.state === "conflict" ? "gpuConflict" : status.state === "unverified" ? "defenderStatusUnverified" : status.state === "resetCommit" ? "resetCommitStatus" : "gpuRecovery");
  const canReset = status && ["none", "applied", "defaults"].includes(status.state);
  const canCompare = status && (status.state === "applied" || status.state === "unverified");
  const needsReapply = !!(canCompare && status.appliedProfile && profileChanged(profile, status.appliedProfile));
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
  return <article className={`entry${dragHandlers ? " entry-draggable" : ""}`} {...dragHandlers} data-drop-scope={dropScope} data-drop-path={item.path} data-drop-favorite={!!item.favorite}>
    <div className="entry-header">
      <ExecutableIcon path={item.path} language={lang} launch={() => launch(item.path, entryName)} disabled={launchDisabled} />
      <div className="entry-content">
        <div className="entry-name">
          <h3 title={nameFailed ? `NAME_READ: ${t(lang, "nameError")}` : entryName}>{entryName}</h3>
          <button className="icon-button entry-launch" disabled={launchDisabled} title={launchDisabled ? t(lang, "launchBusy") : t(lang, "launch")} aria-label={t(lang, "launch")} onClick={() => launch(item.path, entryName)}>{launchingPath === item.path ? <LoaderCircle className="spin" size={16}/> : <Play size={16}/>}</button>
          <div className="entry-menu" ref={menu} onBlur={(event) => { if (!event.currentTarget.contains(event.relatedTarget)) setMenuOpen(false); }}>
            <button ref={menuButton} className="icon-button" title={t(lang, "moreActions")} aria-label={t(lang, "moreActions")} aria-expanded={menuOpen} onClick={() => setMenuOpen((open) => !open)}><MoreVertical size={18}/></button>
            {menuOpen && <div className="entry-menu-panel">
              <button onClick={() => { setMenuOpen(false); void invoke("open_executable_folder", { path: item.path }).catch(showError); }}><FolderOpen size={16}/>{t(lang, "openFolder")}</button>
              {!!item.suppressedWarnings?.length && <button disabled={warningSaving} onClick={() => { setMenuOpen(false); resetEntryWarnings(item.path); }}><RotateCcw size={16}/>{t(lang, "restoreEntryWarnings")}</button>}
              <button className="entry-menu-danger" disabled={busy || launchingPath !== null || status?.state !== "none"} title={status?.state === "none" ? t(lang, "remove") : t(lang, "removeBlocked")} onClick={() => void confirmRemove()}><Trash2 size={16}/>{t(lang, "remove")}</button>
            </div>}
          </div>
        </div>
        <div className="entry-location"><span className="path" title={item.path}>{item.path}</span></div>
      </div>
    </div>
    <div className="entry-meta"><button className={`icon-button entry-favorite${item.favorite ? " selected" : ""}`} title={t(lang, item.favorite ? "removeFavorite" : "addFavorite")} aria-label={`${t(lang, item.favorite ? "removeFavorite" : "addFavorite")}: ${name ?? fileName(item.path)}`} aria-pressed={!!item.favorite} onClick={favorite}><Star size={16} fill={item.favorite ? "currentColor" : "none"}/></button><span className="profile-tag">{profile ? profileName(profile, lang) : t(lang, "noProfile")}</span><span className="status" title={status?.detail ?? undefined}><i />{busy ? <><LoaderCircle className="spin" size={12}/>{t(lang, "working")}</> : statusLabel}</span>{statusFailed && !statusErrorDismissed && <button className="icon-button status-dismiss" title={t(lang, "dismissStatusError")} aria-label={t(lang, "dismissStatusError")} onClick={() => setStatusErrorDismissed(true)}><X size={14}/></button>}</div>
    {!busy && status?.state === "unverified" && status.detail && <p className="profile-change" role="status">{status.detail}</p>}
    {!busy && status?.state === "partial" && <p className="profile-change" role="status">{t(lang, "gpuPartialDetail")}</p>}
    {!busy && canCompare && (needsReapply || !status.appliedProfile) && <p className="profile-change" role="status">{t(lang, needsReapply ? plan ? "profileChanged" : "profileChangedNoActions" : "profileComparisonUnavailable")}</p>}
    {dragHandle}
    {needsReapply && <div className="entry-actions reapply-actions"><button className="entry-optimize" disabled={busy || launchingPath !== null || !plan} title={!plan ? t(lang, "reapplyNoActions") : undefined} onClick={() => void perform("reapply")}><Zap size={14} aria-hidden="true"/>{t(lang, "reapply")}</button></div>}
    <div className="entry-actions entry-profile-actions">
      {status && status.state !== "none"
        ? <button className="entry-restore" disabled={busy || launchingPath !== null} onClick={() => void perform("restore")}><RotateCcw size={14} aria-hidden="true"/>{t(lang, status.state === "resetCommit" ? "finishReset" : "revert")}</button>
        : <button className="entry-optimize" disabled={busy || launchingPath !== null || status?.state !== "none" || !plan} title={!plan ? t(lang, "profileUnsupported") : undefined} onClick={() => void perform("apply")}><Zap size={14} aria-hidden="true"/>{t(lang, "optimize")}</button>}
      <button disabled={busy || launchingPath !== null || !canReset || !hasResetActions} title={!canReset ? t(lang, "resetBlocked") : !hasResetActions ? t(lang, "resetUnsupported") : undefined} onClick={() => void perform("reset")}>{t(lang, "reset")}</button>
    </div>
  </article>;
}
export function Library({ state, update, confirmAction, add, detect, detecting, launch, launchingPath, ensureSaved, showError, showEntryWarning, resetEntryWarnings, warningSaving, activeOperations, claimOperation, releaseOperation }: Props) {
  const lang: Language = state.language;
  const sort = state.librarySort ?? "manual";
  const [names, setNames] = useState<Record<string, { name?: string; failed: boolean }>>({});
  const [collapsed, setCollapsed] = useState<string[]>([]);
  const dragging = useRef<{ paths: string[]; scope: string; favorite: boolean; pointerId: number; x: number; y: number; startX: number; startY: number; active: boolean; source: HTMLElement; preview?: HTMLElement; target?: HTMLElement; plan?: { target: HTMLElement; after: boolean; state: AppState; names: Record<string, string>; items: Executable[] | null }; timer?: ReturnType<typeof setInterval> } | null>(null);
  const library = useRef<HTMLDivElement>(null);
  useEffect(() => () => {
    if (dragging.current?.timer) clearInterval(dragging.current.timer);
    dragging.current?.preview?.remove();
    dragging.current?.source.classList.remove("drag-source");
    dragging.current?.target?.classList.remove("drop-before", "drop-after", "drop-y");
    dragging.current?.target?.style.removeProperty("--drop-line-y");
  }, []);
  useEffect(() => {
    let active = true;
    for (const { path } of state.executables) {
      if (names[path]) continue;
      invoke<string | null>("executable_name", { path })
        .then((name) => { if (active) setNames((current) => ({ ...current, [path]: { name: name ?? undefined, failed: false } })); })
        .catch(() => { if (active) setNames((current) => ({ ...current, [path]: { failed: true } })); });
    }
    return () => { active = false; };
  }, [state.executables]);
  const displayNames = Object.fromEntries(Object.entries(names).map(([path, value]) => [path, value.name ?? ""]));
  const groups = organizeLibrary(state.executables, displayNames, sort, !!state.groupByName, lang);
  const latest = useRef({ state, displayNames, groups, update, lang });
  latest.current = { state, displayNames, groups, update, lang };
  function reorder(paths: string[], targets: string[], after: boolean, scope: string) {
    const current = latest.current;
    if ((current.state.librarySort ?? "manual") !== "manual") return;
    const items = planLibraryMove(current.state.executables, current.displayNames, !!current.state.groupByName, current.lang, scope === "groups" ? null : scope.slice(6), paths, targets, after);
    if (items) current.update({ ...current.state, executables: items });
  }
  function updateDragTarget() {
    const drag = dragging.current;
    if (!drag?.active) return;
    drag.target?.classList.remove("drop-before", "drop-after", "drop-y");
    drag.target?.style.removeProperty("--drop-line-y");
    drag.target = undefined;
    let element = document.elementFromPoint(drag.x, drag.y);
    while (element && element !== library.current) {
      if (element instanceof HTMLElement && element.dataset.dropScope === drag.scope && element.dataset.dropFavorite === String(drag.favorite)) break;
      element = element.parentElement;
    }
    if (!(element instanceof HTMLElement) || !library.current?.contains(element) || element === library.current) {
      drag.plan = undefined;
      return;
    }
    const current = latest.current;
    if ((current.state.librarySort ?? "manual") !== "manual") { drag.plan = undefined; return; }
    const rect = element.getBoundingClientRect();
    const vertical = element.classList.contains("library-group") || (element.parentElement !== null && getComputedStyle(element.parentElement).gridTemplateColumns.split(" ").length === 1);
    const after = vertical ? drag.y >= rect.top + rect.height / 2 : drag.x >= rect.left + rect.width / 2;
    if (!drag.plan || drag.plan.target !== element || drag.plan.after !== after || drag.plan.state !== current.state || drag.plan.names !== current.displayNames) {
      const targets = element.dataset.dropPath ? [element.dataset.dropPath] : current.groups.find((group) => group.key === element.dataset.groupKey)?.entries.map(({ item }) => item.path);
      drag.plan = { target: element, after, state: current.state, names: current.displayNames, items: targets ? planLibraryMove(current.state.executables, current.displayNames, !!current.state.groupByName, current.lang, drag.scope === "groups" ? null : drag.scope.slice(6), drag.paths, targets, after) : null };
    }
    if (!drag.plan.items) return;
    drag.target = element;
    if (vertical) {
      const panel = library.current.getBoundingClientRect();
      const edge = after ? rect.bottom : rect.top;
      element.style.setProperty("--drop-line-y", `${Math.max(panel.top + 4, Math.min(panel.bottom - 8, edge)) - rect.top}px`);
      element.classList.add("drop-y");
    }
    element.classList.add(after ? "drop-after" : "drop-before");
  }
  function endDrag(commit: boolean) {
    const drag = dragging.current;
    if (!drag) return;
    if (drag.timer) clearInterval(drag.timer);
    updateDragTarget();
    drag.target?.classList.remove("drop-before", "drop-after", "drop-y");
    drag.target?.style.removeProperty("--drop-line-y");
    drag.source.classList.remove("drag-source");
    drag.preview?.remove();
    if (commit && drag.active && drag.target && drag.plan?.items) {
      const current = latest.current;
      current.update({ ...current.state, executables: drag.plan.items });
    }
    dragging.current = null;
  }
  // Pointer dragging leaves Tauri's native file-drop handler enabled on Windows.
  function dragHandlers(paths: string[], scope: string, favorite: boolean, fromHandle: boolean): DragHandlers {
    return { onPointerDown: (event) => {
      if (fromHandle) event.stopPropagation();
      if (event.button !== 0 || dragging.current) return;
      if (!fromHandle && event.target instanceof Element && event.target.closest("button, input, select, textarea, a")) return;
      const source = fromHandle ? event.currentTarget.closest<HTMLElement>(".entry, .library-group-header") : event.currentTarget;
      if (!source) return;
      event.currentTarget.setPointerCapture(event.pointerId);
      dragging.current = { paths, scope, favorite, source, pointerId: event.pointerId, x: event.clientX, y: event.clientY, startX: event.clientX, startY: event.clientY, active: false };
    }, onPointerMove: (event) => {
      if (fromHandle) event.stopPropagation();
      const drag = dragging.current;
      if (!drag || drag.pointerId !== event.pointerId) return;
      drag.x = event.clientX;
      drag.y = event.clientY;
      if (!drag.active && Math.hypot(drag.x - drag.startX, drag.y - drag.startY) >= 5) {
        drag.active = true;
        const rect = drag.source.getBoundingClientRect();
        const preview = drag.source.cloneNode(true) as HTMLElement;
        preview.classList.add("drag-preview");
        preview.setAttribute("aria-hidden", "true");
        preview.setAttribute("inert", "");
        preview.querySelectorAll("[id]").forEach((element) => element.removeAttribute("id"));
        Object.assign(preview.style, { left: `${rect.left}px`, top: `${rect.top}px`, width: `${rect.width}px`, height: `${rect.height}px` });
        document.body.appendChild(preview);
        drag.preview = preview;
        drag.source.classList.add("drag-source");
        drag.timer = setInterval(() => {
          const current = dragging.current;
          const panel = library.current;
          if (!current || !panel) return;
          const rect = panel.getBoundingClientRect();
          if (current.y < rect.top + 32) panel.scrollTop -= 12;
          else if (current.y > rect.bottom - 32) panel.scrollTop += 12;
          updateDragTarget();
        }, 40);
      }
      if (drag.preview) drag.preview.style.transform = `translate(${drag.x - drag.startX}px, ${drag.y - drag.startY}px)`;
      updateDragTarget();
    }, onPointerUp: (event) => { if (fromHandle) event.stopPropagation(); const drag = dragging.current; if (drag?.pointerId === event.pointerId) { drag.x = event.clientX; drag.y = event.clientY; endDrag(true); } },
    onPointerCancel: (event) => { if (fromHandle) event.stopPropagation(); if (dragging.current?.pointerId === event.pointerId) endDrag(false); },
    onLostPointerCapture: (event) => { if (fromHandle) event.stopPropagation(); if (dragging.current?.pointerId === event.pointerId) endDrag(false); } };
  }
  function handle(paths: string[], scope: string, favorite: boolean, peers: { paths: string[]; favorite: boolean }[], label: string, name: string) {
    const index = peers.findIndex((peer) => peer.paths[0] === paths[0]);
    return <button className="drag-handle" title={`${label}: ${name} · ← →`} aria-label={`${label}: ${name} · ${t(lang, "moveEarlier")} / ${t(lang, "moveLater")}`} {...dragHandlers(paths, scope, favorite, true)} onKeyDown={(event) => {
      const direction = event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : 0;
      if (!direction) return;
      event.preventDefault();
      const peer = peers[index + direction];
      if (peer && peer.favorite === favorite) reorder(paths, peer.paths, direction > 0, scope);
    }}><GripVertical size={16}/>{label}</button>;
  }
  const groupPeers = groups.map((group) => ({ paths: group.entries.map(({ item }) => item.path), favorite: group.entries.some(({ item }) => !!item.favorite) }));
  function entryCard(entry: LibraryEntry, group: LibraryGroup, grouped: boolean) {
    const { item } = entry;
    const scope = grouped ? `group:${group.key}` : "groups";
    const peers = grouped ? group.entries.map(({ item: member }) => ({ paths: [member.path], favorite: !!member.favorite })) : groupPeers;
    return <Entry key={item.path} item={item} name={names[item.path]?.name} nameFailed={!!names[item.path]?.failed} state={state} launch={launch} launchingPath={launchingPath} confirmAction={confirmAction} ensureSaved={ensureSaved} showError={showError} showEntryWarning={showEntryWarning} resetEntryWarnings={resetEntryWarnings} warningSaving={warningSaving} activeOperations={activeOperations} claimOperation={claimOperation} releaseOperation={releaseOperation}
      favorite={() => update({ ...state, executables: state.executables.map((member) => member.path === item.path ? { ...member, favorite: !member.favorite } : member) })}
      remove={() => update({ ...state, executables: state.executables.filter((member) => member.path !== item.path) })}
      dragHandle={sort === "manual" ? handle([item.path], scope, !!item.favorite, peers, t(lang, "dragItem"), entry.name) : undefined}
      dragHandlers={sort === "manual" ? dragHandlers([item.path], scope, !!item.favorite, false) : undefined}
      dropScope={sort === "manual" ? scope : undefined}/>;
  }
  return <section className="page"><header className="page-header"><div><span className="eyebrow">GAME OPTIMIZER / {t(lang, "library").toUpperCase()}</span><h1>{t(lang, "title")}</h1><p>{t(lang, "subtitle")}</p></div><div className="library-add-actions"><button className="secondary detect-button" disabled={detecting} title={t(lang, detecting ? "detecting" : "detectShort")} aria-label={t(lang, detecting ? "detecting" : "detectShort")} onClick={detect}>{detecting ? <LoaderCircle className="spin" size={18}/> : <Search size={18}/>}</button><button className="primary" onClick={add}><Plus size={18}/>{t(lang, "add")}</button></div></header>
    {!state.libraryNoticeDismissed && <DismissibleNotice language={lang} dismiss={() => update({ ...state, libraryNoticeDismissed: true })}>{t(lang, "gpuPreview")}</DismissibleNotice>}
    <div className="toolbar"><span className="count">{state.executables.length} {t(lang, "library").toLowerCase()}</span><div className="toolbar-actions"><label>{t(lang, "sortLibrary")} <select value={sort} onChange={(event) => update({ ...state, librarySort: event.target.value as LibrarySort })}><option value="az">{t(lang, "sortAZ")}</option><option value="za">{t(lang, "sortZA")}</option><option value="manual">{t(lang, "sortManual")}</option></select></label><label className="group-toggle"><input type="checkbox" checked={!!state.groupByName} onChange={(event) => update({ ...state, groupByName: event.target.checked })}/>{t(lang, "groupNames")}</label></div></div>
    <div ref={library} className="library-items scroll-panel" tabIndex={0} role="region" aria-label={t(lang, "library")}>
      {state.executables.length === 0 ? <div className="empty"><div className="empty-icon"><Gamepad2 size={32}/></div><h2>{t(lang, "empty")}</h2><p>{t(lang, "emptyText")}</p><button className="secondary" onClick={add}><Plus size={16}/>{t(lang, "add")}</button></div> : <div className="entry-grid">{groups.map((group) => {
        if (group.entries.length === 1) return entryCard(group.entries[0], group, false);
        const paths = group.entries.map(({ item }) => item.path);
        const favorite = group.entries.some(({ item }) => !!item.favorite);
        const isCollapsed = collapsed.includes(group.key);
        return <section className="library-group" key={group.key} data-drop-scope={sort === "manual" ? "groups" : undefined} data-drop-favorite={favorite} data-group-key={group.key}>
          <div className="library-group-header"><button className="group-heading" aria-expanded={!isCollapsed} onClick={() => setCollapsed((current) => isCollapsed ? current.filter((key) => key !== group.key) : [...current, group.key])}>{isCollapsed ? <ChevronRight size={18}/> : <ChevronDown size={18}/>}<strong>{group.name}</strong><span>{group.entries.length}</span></button>{sort === "manual" && handle(paths, "groups", favorite, groupPeers, t(lang, "dragGroup"), group.name)}</div>
          {!isCollapsed && <div className="entry-grid">{group.entries.map((entry) => entryCard(entry, group, true))}</div>}
        </section>;
      })}</div>}
    </div>
  </section>;
}
