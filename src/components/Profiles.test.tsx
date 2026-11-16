import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { initialState } from "../model";
import { Profiles } from "./Profiles";

describe("Profiles", () => {
  it("keeps the Defender checkbox enabled in both states", () => {
    const state = initialState();
    const renderCheckbox = () => renderToStaticMarkup(
      <Profiles state={state} update={() => {}} showError={() => {}} />,
    ).match(/<input type="checkbox"[^>]*>/)?.[0];

    expect(renderCheckbox()).not.toContain("disabled");
    expect(renderCheckbox()).not.toContain("checked");
    state.profiles[0].defender = true;
    expect(renderCheckbox()).not.toContain("disabled");
    expect(renderCheckbox()).toContain("checked");
  });
});
