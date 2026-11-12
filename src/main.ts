import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

async function start() {
  // Hors de Tauri (navigateur), en développement seulement : cœur Rust simulé.
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    const { installMock } = await import("./dev-mock");
    installMock();
  }
  mount(App, { target: document.getElementById("app")! });
}

start();
