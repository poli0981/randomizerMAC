import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

async function start() {
  // Outside Tauri (plain browser during development) use fake backend data.
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    const { installDevMock } = await import("./lib/devmock");
    installDevMock();
  }
  const target = document.getElementById("app");
  if (!target) throw new Error("#app element missing");
  mount(App, { target });
}

void start();
