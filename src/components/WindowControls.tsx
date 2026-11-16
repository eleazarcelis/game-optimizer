import { useEffect, useState } from "react";
import { getCurrentWindow, type Window as TauriWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "lucide-react";
import type { Language } from "../model";
import { translate as t } from "../i18n";

type Props = { language: Language; showError: (error: unknown) => void };
const directions: Parameters<TauriWindow["startResizeDragging"]>[0][] = ["North", "NorthEast", "East", "SouthEast", "South", "SouthWest", "West", "NorthWest"];

export function WindowControls({ language, showError }: Props) {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    let active = true;
    let sequence = 0;
    let unlisten: (() => void) | undefined;
    const appWindow = getCurrentWindow();
    function refresh() {
      const current = ++sequence;
      appWindow.isMaximized().then((value) => {
        if (active && current === sequence) setMaximized(value);
      }).catch((error: unknown) => { if (active) showError(error); });
    }
    appWindow.onResized(refresh).then((stop) => {
      if (!active) stop();
      else { unlisten = stop; refresh(); }
    }).catch((error: unknown) => { if (active) showError(error); });
    return () => { active = false; unlisten?.(); };
  }, [showError]);

  function run(action: () => Promise<void>) {
    void action().catch(showError);
  }

  const maximizeLabel = t(language, maximized ? "restoreWindow" : "maximizeWindow");
  return <>
    {!maximized && directions.map((direction) => <div key={direction} className={`window-resize window-resize-${direction.toLowerCase()}`} onMouseDown={(event) => {
      if (event.button === 0) run(() => getCurrentWindow().startResizeDragging(direction));
    }} />)}
    <div className="window-bar">
      <div className="window-drag" data-tauri-drag-region />
      <div className="window-controls">
        <button title={t(language, "minimizeWindow")} aria-label={t(language, "minimizeWindow")} onClick={() => run(() => getCurrentWindow().minimize())}><Minus size={16}/></button>
        <button title={maximizeLabel} aria-label={maximizeLabel} onClick={() => run(() => getCurrentWindow().toggleMaximize())}>{maximized ? <Copy size={14}/> : <Square size={14}/>}</button>
        <button className="window-close" title={t(language, "close")} aria-label={t(language, "close")} onClick={() => run(() => getCurrentWindow().close())}><X size={16}/></button>
      </div>
    </div>
  </>;
}
