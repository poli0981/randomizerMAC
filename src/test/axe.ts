// Helpers for the accessibility tests (src/lib/a11y.test.ts): axe-core in jsdom.
import axe from "axe-core";

/**
 * The axe-core violations under `root`, one line each ("rule: help (targets)"). jsdom has no
 * layout engine, so colour contrast cannot be measured here: it is checked by hand (see
 * ACCESSIBILITY.md).
 */
export async function axeViolations(root: Element = document.body): Promise<string[]> {
  const result = await axe.run(root, {
    rules: { "color-contrast": { enabled: false } },
    resultTypes: ["violations"],
  });
  return result.violations.map(
    (v) => `${v.id}: ${v.help} (${v.nodes.map((n) => n.target.join(" ")).join(", ")})`,
  );
}

/** What jsdom lacks for the app to render: modal dialogs and element scrolling. */
export function polyfillJsdom(): void {
  const dialog = HTMLDialogElement.prototype as Partial<HTMLDialogElement>;
  if (typeof dialog.showModal !== "function") {
    dialog.showModal = function (this: HTMLDialogElement) {
      this.setAttribute("open", "");
    };
    dialog.close = function (this: HTMLDialogElement) {
      this.removeAttribute("open");
      this.dispatchEvent(new Event("close"));
    };
  }
  // axe-core probes canvas support; jsdom only logs "not implemented".
  HTMLCanvasElement.prototype.getContext = () => null;
  const element = Element.prototype as Partial<Element>;
  if (typeof element.scrollTo !== "function") {
    element.scrollTo = function () {};
  }
}
