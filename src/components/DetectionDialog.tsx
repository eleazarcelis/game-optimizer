import { useEffect, useRef, useState } from "react";
import type { AppState } from "../model";
import { containsPath } from "../model";
import { translate as t, type Key } from "../i18n";
import { initialGameSelection, type DetectedSelection, type DetectionIssue, type DetectionResult } from "../detection";

type Props = { state: AppState; result: DetectionResult; saving: boolean; error: string; cancel: () => void; add: (selected: DetectedSelection[]) => void };
const issueKeys: Record<string, Key> = {
  steam_missing: "detectSteamMissing", registry: "detectRegistryError", invalid_steam: "detectInvalidSteam",
  library_metadata: "detectLibraryMetadata", library: "detectLibraryError", manifest: "detectManifestError",
  directory: "detectDirectoryError", link_skipped: "detectLinkSkipped", scan_limit: "detectScanLimit",
  issues_truncated: "detectIssuesTruncated",
};

function issueText(language: AppState["language"], issue: DetectionIssue): string {
  return `${t(language, issueKeys[issue.kind] ?? "detectUnknownIssue")}${issue.path ? `: ${issue.path}` : ""}`;
}

export function DetectionDialog({ state, result, saving, error, cancel, add }: Props) {
  const lang = state.language;
  const dialog = useRef<HTMLDialogElement>(null);
  const [steamSelected, setSteamSelected] = useState(!!result.steam && !containsPath(state.executables, result.steam));
  const [selected, setSelected] = useState(() => result.games.map((game) => initialGameSelection(game, state.executables)));
  useEffect(() => {
    const element = dialog.current;
    if (!element) return;
    element.showModal();
    return () => { if (element.open) element.close(); };
  }, []);
  const choices: DetectedSelection[] = [
    ...(steamSelected && result.steam && !containsPath(state.executables, result.steam) ? [{ path: result.steam, kind: "steam" as const }] : []),
    ...selected.filter((path) => path && !containsPath(state.executables, path)).map((path) => ({ path, kind: "game" as const })),
  ];
  return <dialog ref={dialog} className="confirm-dialog detection-dialog" aria-labelledby="detection-title" onCancel={(event) => { event.preventDefault(); if (!saving) cancel(); }}>
    <h2 id="detection-title">{t(lang, "detectResults")}</h2>
    <p>{t(lang, "detectReview")}</p>
    <div className="detection-results scroll-panel">
      {result.steam && <label className="detection-steam">
        <input type="checkbox" checked={steamSelected && !containsPath(state.executables, result.steam)} disabled={saving || containsPath(state.executables, result.steam)} onChange={(event) => setSteamSelected(event.target.checked)}/>
        <span><strong>Steam</strong><small>{result.steam}{containsPath(state.executables, result.steam) ? ` — ${t(lang, "detectAlreadyAdded")}` : ""}</small></span>
      </label>}
      {result.games.map((game, index) => <label className="detection-game" key={`${game.app_id}-${index}`}>
        <strong>{game.name}</strong>
        <select value={selected[index]} disabled={saving || game.candidates.length === 0} onChange={(event) => setSelected((current) => current.map((path, i) => i === index ? event.target.value : path))}>
          <option value="">{t(lang, game.candidates.length ? "detectDoNotAdd" : "detectNoExecutable")}</option>
          {game.candidates.map((path) => <option value={path} key={path} disabled={containsPath(state.executables, path)}>{path}{containsPath(state.executables, path) ? ` — ${t(lang, "detectAlreadyAdded")}` : ""}</option>)}
        </select>
      </label>)}
      {!result.steam && result.games.length === 0 && <p>{t(lang, "detectNoResults")}</p>}
      {result.issues.length > 0 && <div className="detection-issues" role="status"><strong>{t(lang, "detectIssues")}</strong><ul>{result.issues.map((issue, index) => <li key={index}>{issueText(lang, issue)}</li>)}</ul></div>}
      {error && <p className="detection-error" role="alert">{error}</p>}
    </div>
    <div className="modal-actions"><button className="secondary" disabled={saving} onClick={cancel}>{t(lang, "cancel")}</button><button className="primary" disabled={saving || choices.length === 0} onClick={() => add(choices)}>{t(lang, saving ? "working" : "detectAddSelected")}</button></div>
  </dialog>;
}
