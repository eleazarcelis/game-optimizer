import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { initialState } from "../model";
import { Settings } from "./Settings";
import type { UpdateStatus } from "../updates";

describe("About update controls", () => {
  it("shows a download button only for a newer release with an uploaded executable", () => {
    const state = initialState();
    state.language = "es";
    const render = (updateStatus: UpdateStatus) => renderToStaticMarkup(
      <Settings state={state} update={() => {}} showError={() => {}} initialTab="about" updateStatus={updateStatus} checkUpdates={() => {}} />,
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
      <Settings state={state} update={() => {}} showError={() => {}} initialTab="about" updateStatus={{ kind: "idle" }} checkUpdates={() => {}} />,
    );
    expect(html).toContain("GPL-3.0-or-later");
    expect(html).toContain("Ver texto de la licencia (inglés)");
    expect(html).toContain("GNU GENERAL PUBLIC LICENSE");
    expect(html).toContain("github.com/eleazarcelis/game-optimizer");
  });
  it("explains the Windows process focus and graphics limitation in both languages", () => {
    const state = initialState();
    const render = () => renderToStaticMarkup(
      <Settings state={state} update={() => {}} showError={() => {}} initialTab="about" updateStatus={{ kind: "idle" }} checkUpdates={() => {}} />,
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
      <Settings state={state} update={() => {}} showError={() => {}} initialTab="about" updateStatus={{ kind: "idle" }} checkUpdates={() => {}} />,
    );
    expect(render()).toContain("Report a bug or suggest an improvement");
    state.language = "es";
    expect(render()).toContain("Reportar errores o sugerir mejoras");
  });
});
