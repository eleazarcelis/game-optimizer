import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { AlertCircle, LoaderCircle } from "lucide-react";
import { Sidebar } from "./components/Sidebar";
import { Library } from "./components/Library";
import { Settings } from "./components/Settings";
import { GlobalSettings } from "./components/GlobalSettings";
import { ConfirmationDialog } from "./components/ConfirmationDialog";
import { WindowControls } from "./components/WindowControls";
import { localizeError } from "./errors";
import { launchWindowAction, setEntryWarnings, warningId, warningSuppressed, type EntryWarning, type WarningNotification } from "./entry-warnings";
import { containsPath, initialState, isAppState, isExe, type AppState, type Language } from "./model";
import { dontShowAgainText, profileName, translate as t } from "./i18n";
import { checkForUpdates, type UpdateStatus } from "./updates";
import "./App.css";

type AppError = { code?: string; message?: string };
const clientCodes = new Set(["UPDATE_CHECK", "WINDOW_ACTION", "REAPPLY_FAILED"]);
function errorText(language: Language, error: unknown): string {
  if (error && typeof error === "object" && "message" in error) {
    const e = error as AppError;
    if (!e.code || clientCodes.has(e.code)) return `${e.code ?? "UNKNOWN"}: ${e.message ?? ""}`;
    return localizeError(language, e.code, e.message ?? "");
  }
  return `${t(language, "errUnknown")}`;
}

function App() {
  const [state, setState] = useState<AppState>();
  const stateRef = useRef<AppState | undefined>(undefined);
  const pendingWrites = useRef<Promise<void>>(Promise.resolve());
  const [page, setPage] = useState<"library" | "settings" | "global">("library");
  const [aboutRequest, setAboutRequest] = useState(0);
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>({ kind: "idle" });
  const [updateNotification, setUpdateNotification] = useState(false);
  const updateCheckInFlight = useRef(false);
  const startupUpdateCheckDone = useRef(false);
  const [error, setError] = useState("");
  const [warnings, setWarnings] = useState<WarningNotification[]>([]);
  const [warningSaving, setWarningSaving] = useState(false);
  const warningSavingRef = useRef(false);
  const [loadFailed, setLoadFailed] = useState(false);
  const [loadAttempt, setLoadAttempt] = useState(0);
  const [pendingPaths, setPendingPaths] = useState<string[]>([]);
  const [chosenProfile, setChosenProfile] = useState("");
  const [busy, setBusy] = useState(false);
  const activePaths = useRef(new Set<string>());
  const launching = useRef(false);
  const [launchingPath, setLaunchingPath] = useState<string | null>(null);
  const [activeOperations, setActiveOperations] = useState<string[]>([]);
  const [confirmation, setConfirmation] = useState<string | null>(null);
  const confirmationResolver = useRef<((confirmed: boolean) => void) | null>(null);
  const showError = useCallback((error: unknown) => {
    const language = stateRef.current?.language ?? "en";
    if (error && typeof error === "object" && "warning" in error) setWarnings((current) => [...current, { message: String((error as { warning: unknown }).warning) }]);
    else setError(errorText(language, error));
  }, []);
  const ensureSaved = useCallback(() => pendingWrites.current, []);
  const confirmAction = useCallback((message: string): Promise<boolean> => {
    if (confirmationResolver.current) return Promise.resolve(false);
    return new Promise((resolve) => {
      confirmationResolver.current = resolve;
      setConfirmation(message);
    });
  }, []);
  const finishConfirmation = useCallback((confirmed: boolean) => {
    const resolve = confirmationResolver.current;
    confirmationResolver.current = null;
    setConfirmation(null);
    resolve?.(confirmed);
  }, []);
  useEffect(() => () => {
    confirmationResolver.current?.(false);
    confirmationResolver.current = null;
  }, []);
  const claimOperation = useCallback((path: string) => {
    const key = path.toLowerCase();
    if (launching.current || activePaths.current.has(key)) return false;
    activePaths.current.add(key);
    setActiveOperations([...activePaths.current]);
    return true;
  }, []);
  const releaseOperation = useCallback((path: string) => {
    activePaths.current.delete(path.toLowerCase());
    setActiveOperations([...activePaths.current]);
  }, []);

  const update = useCallback((next: AppState) => {
    stateRef.current = next;
    setState(next);
    pendingWrites.current = pendingWrites.current.catch(() => undefined).then(() =>
      invoke<void>("save_state", { state: next }).catch((e: unknown) => { setError(errorText(stateRef.current?.language ?? "en", e)); throw e; }),
    );
    void pendingWrites.current.catch(() => undefined);
  }, []);

  const showEntryWarning = useCallback((path: string, name: string, id: string, message: string): boolean => {
    const entry = { path, name, id };
    if (warningSuppressed(stateRef.current, entry)) return false;
    const exists = stateRef.current?.executables.some((item) => item.path.toLowerCase() === path.toLowerCase());
    setWarnings((current) => [...current, { message, entry: exists ? entry : undefined }]);
    return true;
  }, []);

  const changeWarningPreference = useCallback(async (path: string, change: (ids: string[]) => string[]) => {
    if (warningSavingRef.current) return false;
    const state = stateRef.current;
    const item = state?.executables.find((entry) => entry.path.toLowerCase() === path.toLowerCase());
    if (!state || !item) return false;
    const previous = item.suppressedWarnings ?? [];
    warningSavingRef.current = true;
    setWarningSaving(true);
    update(setEntryWarnings(state, path, change(previous)));
    try {
      await ensureSaved();
      return true;
    } catch {
      const current = stateRef.current;
      if (current) update(setEntryWarnings(current, path, previous));
      return false;
    } finally {
      warningSavingRef.current = false;
      setWarningSaving(false);
    }
  }, [ensureSaved, update]);

  const toggleWarning = useCallback(async (entry: EntryWarning, suppressed: boolean) => {
    const saved = await changeWarningPreference(entry.path, (ids) =>
      suppressed ? [...new Set([...ids, entry.id])] : ids.filter((id) => id !== entry.id));
    if (saved && suppressed) setWarnings((current) => current.filter((warning, index) =>
      index === 0 || !warning.entry || warning.entry.path.toLowerCase() !== entry.path.toLowerCase() || warning.entry.id !== entry.id));
  }, [changeWarningPreference]);

  const resetEntryWarnings = useCallback((path: string) => {
    void changeWarningPreference(path, () => []);
  }, [changeWarningPreference]);

  const checkUpdates = useCallback(async (automatic: boolean) => {
    if (updateCheckInFlight.current) return;
    updateCheckInFlight.current = true;
    setUpdateStatus({ kind: "checking" });
    setUpdateNotification(false);
    try {
      const result = await checkForUpdates(await getVersion());
      setUpdateStatus(result ? { kind: "available", ...result } : { kind: "current" });
      if (automatic && result && stateRef.current?.automaticUpdateChecks === true) setUpdateNotification(true);
    } catch (e) {
      setUpdateStatus({ kind: "failed" });
      showError({ code: "UPDATE_CHECK", message: `${t(stateRef.current?.language ?? "en", "updateCheckFailed")} ${errorText(stateRef.current?.language ?? "en", e)}` });
    } finally {
      updateCheckInFlight.current = false;
    }
  }, [showError]);

  useEffect(() => {
    if (!state || startupUpdateCheckDone.current) return;
    startupUpdateCheckDone.current = true;
    if (state.automaticUpdateChecks === true) void checkUpdates(true);
  }, [state, checkUpdates]);

  useEffect(() => {
    let active = true;
    invoke<unknown>("load_state").then((saved) => {
      if (!active) return;
      if (saved !== null && !isAppState(saved)) { setError(localizeError("en", "DATA_INVALID", "")); setLoadFailed(true); return; }
      const next = saved ?? initialState();
      stateRef.current = next;
      setState(next);
    }).catch((e: unknown) => { if (active) { setError(errorText(stateRef.current?.language ?? "en", e)); setLoadFailed(true); } });
    return () => { active = false; };
  }, [loadAttempt]);

  const prepare = useCallback(async (paths: string[]) => {
    const current = stateRef.current;
    if (!current) return;
    if (paths.some((path) => !isExe(path))) { setError(`INVALID_EXECUTABLE: ${t(current.language, "invalid")}`); return; }
    if (paths.some((path) => containsPath(current.executables, path)) ||
        new Set(paths.map((path) => path.toLocaleLowerCase("en-US"))).size !== paths.length) {
      setError(`DUPLICATE: ${t(current.language, "duplicate")}`); return;
    }
    setBusy(true);
    try {
      await Promise.all(paths.map((path) => invoke<string>("inspect_executable", { path })));
      setChosenProfile(current.profiles[0]?.id ?? "");
      setPendingPaths(paths);
    } catch (e) { setError(errorText(stateRef.current?.language ?? "en", e)); } finally { setBusy(false); }
  }, []);

  useEffect(() => {
    let cancelled = false;
    let remove: (() => void) | undefined;
    getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "drop" && event.payload.paths.length) {
        void prepare(event.payload.paths);
      }
    }).then((unlisten) => { if (cancelled) unlisten(); else remove = unlisten; })
      .catch((e: unknown) => setError(errorText(stateRef.current?.language ?? "en", e)));
    return () => { cancelled = true; remove?.(); };
  }, [prepare]);

  async function addFromDialog() {
    try {
      const paths = await open({ multiple: true, filters: [{ name: "Windows executable", extensions: ["exe"] }] });
      if (paths?.length) await prepare(paths);
    } catch (e) { setError(errorText(stateRef.current?.language ?? "en", e)); }
  }

  async function launch(path: string, name: string) {
    if (launching.current || activePaths.current.size > 0) return;
    launching.current = true;
    setLaunchingPath(path);
    try {
      await ensureSaved();
      const warning = await invoke<"priorityUnavailable" | "jobUnavailable" | "packagedPriorityUnavailable" | null>("launch_executable", { path });
      const warningShown = warning ? showEntryWarning(path, name, warningId("launch", warning), t(stateRef.current?.language ?? "en", warning === "priorityUnavailable" ? "launchPriorityUnavailable" : warning === "packagedPriorityUnavailable" ? "launchPackagedPriorityUnavailable" : "launchJobUnavailable")) : false;
      try {
        await ensureSaved();
        const action = launchWindowAction(stateRef.current?.launchBehavior ?? "minimize", warningShown);
        if (action === "close") await getCurrentWindow().close();
        else if (action === "minimize") await getCurrentWindow().minimize();
      } catch (e) {
        showError({ code: "WINDOW_ACTION", message: `${t(stateRef.current?.language ?? "en", "launchSucceededWindowFailed")} ${errorText(stateRef.current?.language ?? "en", e)}` });
      }
    } catch (e) {
      showError(e);
    } finally {
      launching.current = false;
      setLaunchingPath(null);
    }
  }

  const lang = state?.language ?? "en";
  const errorNotification = error && <div className="toast" role="alert"><AlertCircle size={20}/><div className="toast-message scroll-panel"><strong>{t(lang, "error")}</strong><p>{error}</p></div><button className="icon-button" onClick={() => setError("")} title={t(lang, "close")} aria-label={t(lang, "close")}>×</button></div>;
  const currentWarning = warnings[0];
  const preferenceEntry = currentWarning?.entry;
  const warningNotification = currentWarning && <div className="toast toast-warning" role="status"><AlertCircle size={20}/><div className="toast-message scroll-panel"><strong>{t(lang, "warning")}</strong>{preferenceEntry && <p title={preferenceEntry.path}>{t(lang, "warningEntry")}: {preferenceEntry.name}</p>}<p style={{ whiteSpace: "pre-line" }}>{currentWarning.message}</p>{preferenceEntry && <label className="warning-preference"><input type="checkbox" disabled={warningSaving} checked={warningSuppressed(state, preferenceEntry)} onChange={(event) => void toggleWarning(preferenceEntry, event.target.checked)}/>{dontShowAgainText(lang, preferenceEntry.name)}</label>}</div><button className="icon-button" disabled={warningSaving} onClick={() => setWarnings((current) => current.slice(1))} title={t(lang, "close")} aria-label={t(lang, "close")}>×</button></div>;
  if (!state) return <><WindowControls language={lang} showError={showError}/><div className="loading">{loadFailed ? <div className="load-failed"><p>{t(lang, "dataUnavailable")}</p><button className="secondary" onClick={() => { setError(""); setLoadFailed(false); setLoadAttempt((attempt) => attempt + 1); }}>{t(lang, "retry")}</button></div> : <><LoaderCircle className="spin"/>{t(lang, "loading")}</>}</div>{errorNotification}{warningNotification}</>;
  return <><WindowControls language={lang} showError={showError}/><div className="app-shell"><Sidebar page={page} setPage={setPage} language={lang} showError={showError} />
    <main className="content">{page === "library" ? <Library state={state} update={update} add={() => void addFromDialog()} launch={(path, name) => void launch(path, name)} launchingPath={launchingPath} confirmAction={confirmAction} ensureSaved={ensureSaved} showError={showError} showEntryWarning={showEntryWarning} resetEntryWarnings={resetEntryWarnings} warningSaving={warningSaving} activeOperations={activeOperations} claimOperation={claimOperation} releaseOperation={releaseOperation} /> : page === "settings" ? <Settings key={aboutRequest} initialTab={aboutRequest ? "about" : "general"} state={state} update={update} showError={setError} updateStatus={updateStatus} checkUpdates={() => void checkUpdates(false)}/> : <GlobalSettings state={state} update={update} confirmAction={confirmAction} ensureSaved={ensureSaved} showError={showError} activeOperations={activeOperations} launching={launchingPath !== null} claimOperation={claimOperation} releaseOperation={releaseOperation}/>}</main>
    {busy && <div className="overlay"><div className="busy"><LoaderCircle className="spin"/>{t(lang, "loading")}</div></div>}
    {pendingPaths.length > 0 && <div className="overlay"><div className="modal" role="dialog" aria-modal="true" aria-labelledby="modal-title"><h2 id="modal-title">{t(lang, "selectProfile")}</h2><p className="modal-path scroll-panel" tabIndex={0}>{pendingPaths.join(", ")}</p><select value={chosenProfile} onChange={(e) => setChosenProfile(e.target.value)}>{state.profiles.map((profile) => <option key={profile.id} value={profile.id}>{profileName(profile, lang)}</option>)}</select><div className="modal-actions"><button className="secondary" onClick={() => setPendingPaths([])}>{t(lang, "cancel")}</button><button className="primary" disabled={!chosenProfile} onClick={() => { update({ ...state, executables: [...state.executables, ...pendingPaths.map((path) => ({ path, profileId: chosenProfile }))] }); setPendingPaths([]); }}>{t(lang, "confirm")}</button></div></div></div>}
    {confirmation !== null && <ConfirmationDialog message={confirmation} language={lang} onDecision={finishConfirmation} />}
    {updateNotification && updateStatus.kind === "available" && <div className="update-toast" role="status"><div><strong>{t(lang, "updateAvailable")} {updateStatus.tag}</strong><p>{t(lang, "updateNotice")}</p></div><button className="secondary" onClick={() => { setUpdateNotification(false); setAboutRequest((request) => request + 1); setPage("settings"); }}>{t(lang, "viewUpdate")}</button><button className="icon-button" aria-label={t(lang, "close")} onClick={() => setUpdateNotification(false)}>×</button></div>}
    {errorNotification}{warningNotification}
  </div></>;
}
export default App;
