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
});
