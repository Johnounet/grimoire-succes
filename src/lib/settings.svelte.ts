// Réglages et état d'interface partagés, enregistrés dans la base via le cœur Rust.
import { api } from "./library";

export const settings = $state({
  /** Floute nom et description des succès cachés non obtenus (activé par défaut). */
  antiSpoiler: true,
  /** Vue de l'espace de travail : tableau libre (grille) ou liste. */
  workspaceView: "board" as "board" | "list",
  /** Panneau de droite (liste complète des succès) ouvert. */
  panelOpen: true,
});

/** État éphémère : un glisser depuis le panneau est en cours (pour montrer les zones d'arrivée). */
export const ui = $state({ draggingFromPanel: false });

export async function loadSettings() {
  const [spoiler, view, panel] = await Promise.all(
    (["anti_spoiler", "workspace_view", "panel_open"] as const).map((k) => api.getSetting(k).catch(() => null)),
  );
  settings.antiSpoiler = spoiler !== "0";
  // « columns » (ancienne vue Colonnes, retirée) devient le tableau
  settings.workspaceView = view === "list" ? "list" : "board";
  settings.panelOpen = panel !== "0";
}

export async function setAntiSpoiler(on: boolean) {
  settings.antiSpoiler = on;
  await api.setSetting("anti_spoiler", on ? "1" : "0");
}

export async function setWorkspaceView(view: "board" | "list") {
  settings.workspaceView = view;
  await api.setSetting("workspace_view", view);
}

export async function setPanelOpen(open: boolean) {
  settings.panelOpen = open;
  await api.setSetting("panel_open", open ? "1" : "0");
}
