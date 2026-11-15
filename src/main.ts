import { mount } from "svelte";
import "./app.css";

async function start() {
  // Maquette de la vue unique des succès (développement seulement, « ?maquette »)
  if (import.meta.env.DEV && new URLSearchParams(location.search).has("maquette")) {
    const { default: Maquette } = await import("./maquette/Maquette.svelte");
    mount(Maquette, { target: document.getElementById("app")! });
    return;
  }
  // Hors de Tauri (navigateur), en développement seulement : cœur Rust simulé.
  // « ?compact » dans l'adresse simule la fenêtre compacte.
  let compact = new URLSearchParams(location.search).has("compact");
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    const { installMock } = await import("./dev-mock");
    installMock(compact ? "compact" : "main");
  }
  // Interface importée après le simulateur : elle lit le nom de la fenêtre au chargement
  const { windowLabel } = await import("./lib/sync");
  // Dernier thème utilisé, posé avant le premier affichage
  (await import("./lib/theme.svelte")).applyCachedTheme();
  compact = windowLabel === "compact";
  const target = document.getElementById("app")!;
  if (compact) {
    const { default: Compact } = await import("./Compact.svelte");
    mount(Compact, { target });
  } else {
    const { default: App } = await import("./App.svelte");
    mount(App, { target });
  }
}

start();
