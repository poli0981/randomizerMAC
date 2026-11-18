import { afterEach, describe, expect, it } from "vitest";
import { focusPageHeading, rescueFocus } from "./focus";

afterEach(() => {
  document.body.innerHTML = "";
});

function page() {
  document.body.innerHTML = `
    <h1 tabindex="-1" data-page-heading>Title</h1>
    <button id="a">A</button>
    <button id="b">B</button>`;
  return {
    heading: document.querySelector("h1")!,
    a: document.getElementById("a")!,
    b: document.getElementById("b")!,
  };
}

describe("focus rescue", () => {
  it("moves focus to the page heading when the focused control went away", async () => {
    const { heading, a } = page();
    a.focus();
    a.remove();
    await rescueFocus();
    expect(document.activeElement).toBe(heading);
  });

  it("prefers the given fallback", async () => {
    const { a, b } = page();
    a.focus();
    a.remove();
    await rescueFocus(() => b);
    expect(document.activeElement).toBe(b);
    // A fallback that finds nothing falls back to the heading.
    b.remove();
    await rescueFocus(() => null);
    expect(document.activeElement).toBe(document.querySelector("h1"));
  });

  it("leaves focus alone when it is still on a control", async () => {
    const { a } = page();
    a.focus();
    await rescueFocus();
    expect(document.activeElement).toBe(a);
  });

  it("does nothing without a heading", () => {
    document.body.innerHTML = "<p>no heading</p>";
    focusPageHeading();
    expect(document.activeElement).toBe(document.body);
  });
});
