import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { initialState } from "../model";
import { Settings } from "./Settings";
import type { UpdateStatus } from "../updates";

const detectionProps = { detect: () => {}, detecting: false, detectedSteam: null, detectedGameCount: null };
describe("About update controls", () => {
  it("enables game detection for detected or added Steam and shows the latest game count", () => {
    const state = initialState();
    state.language = "es";
    const render = (detectedSteam: string | null, detectedGameCount: number | null, detecting = false) => renderToStaticMarkup(
      <Settings {...detectionProps} detecting={detecting} detectedSteam={detectedSteam} detectedGameCount={detectedGameCount} state={state} update={() => {}} showError={() => {}} initialTab="detection" updateStatus={{ kind: "idle" }} checkUpdates={() => {}} />,
    );
    const gameButton = (html: string) => html.match(/<button[^>]*aria-label="Detectar juegos de Steam"[^>]*>Buscar<\/button>/)?.[0];
    expect(gameButton(render(null, null))).toContain("disabled");
    expect(render(null, null)).toContain("Primero detecta Steam");
    expect(gameButton(render("C:\\Steam\\steam.exe", 0))).not.toContain("disabled");
    expect(render("C:\\Steam\\steam.exe", 0)).toContain("Juegos detectados: 0");
    expect(render("C:\\Steam\\steam.exe", null)).not.toContain("Agregado a la biblioteca");
    state.executables.push({ path: "c:/Steam/STEAM.EXE", profileId: "launcher" });
    expect(gameButton(render(null, 3))).not.toContain("disabled");
    expect(render(null, 3)).toContain("Agregado a la biblioteca");
    expect(gameButton(render(null, 3, true))).toContain("disabled");
    state.executables = [];
    expect(gameButton(render(null, null))).toContain("disabled");
  });
  it("offers separate Steam and Steam game detection in both languages", () => {
    const state = initialState();
    const render = () => renderToStaticMarkup(
      <Settings {...detectionProps} state={state} update={() => {}} showError={() => {}} initialTab="detection" updateStatus={{ kind: "idle" }} checkUpdates={() => {}} />,
    );
    expect(render()).toContain("Detect Steam games");
    expect(render()).toContain("Detect Steam");
    expect(render()).toContain(">Steam games</strong>");
    state.language = "es";
    expect(render()).toContain("Detectar juegos de Steam");
    expect(render()).toContain("Detectar Steam");
    expect(render()).toContain(">Juegos de Steam</strong>");
  });
  it("shows a download button only for a newer release with an uploaded executable", () => {
    const state = initialState();
    state.language = "es";
    const render = (updateStatus: UpdateStatus) => renderToStaticMarkup(
      <Settings {...detectionProps} state={state} update={() => {}} showError={() => {}} initialTab="about" updateStatus={updateStatus} checkUpdates={() => {}} />,
    );
    expect(render({ kind: "current" })).toContain("Ya tienes la última versión publicada.");
    expect(render({ kind: "available", tag: "v2.2.0", downloadable: false })).toContain("La descarga para Windows aún no está disponible.");
    expect(render({ kind: "available", tag: "v2.2.0", downloadable: false })).not.toContain("Descargar actualización");
    expect(render({ kind: "available", tag: "v2.2.0", downloadable: true })).toContain("Descargar actualización");
    expect(render({ kind: "checking" })).toContain("Buscando…");
  });
  it("includes the GPL license and source information in About", () => {
    const state = initialState();
    state.language = "es";
    const html = renderToStaticMarkup(
      <Settings {...detectionProps} state={state} update={() => {}} showError={() => {}} initialTab="about" updateStatus={{ kind: "idle" }} checkUpdates={() => {}} />,
    );
    expect(html).toContain("GPL-3.0-or-later");
    expect(html).toContain("Ver texto de la licencia (inglés)");
    expect(html).toContain("GNU GENERAL PUBLIC LICENSE");
    expect(html).toContain("github.com/eleazarcelis/game-optimizer");
  });
  it("explains the Windows process focus and graphics limitation in both languages", () => {
    const state = initialState();
    const render = () => renderToStaticMarkup(
      <Settings {...detectionProps} state={state} update={() => {}} showError={() => {}} initialTab="about" updateStatus={{ kind: "idle" }} checkUpdates={() => {}} />,
    );
    expect(render()).toContain("manage how their processes run in Windows");
    expect(render()).toContain("does not change in-game graphics settings");
    state.language = "es";
    expect(render()).toContain("cómo se ejecutan sus procesos en Windows");
    expect(render()).toContain("no cambia los ajustes gráficos del juego");
  });
  it("offers a GitHub issue button in both languages", () => {
    const state = initialState();
    const render = () => renderToStaticMarkup(
      <Settings {...detectionProps} state={state} update={() => {}} showError={() => {}} initialTab="about" updateStatus={{ kind: "idle" }} checkUpdates={() => {}} />,
    );
    expect(render()).toContain("Report a bug or suggest an improvement");
    state.language = "es";
    expect(render()).toContain("Reportar errores o sugerir mejoras");
  });
});
