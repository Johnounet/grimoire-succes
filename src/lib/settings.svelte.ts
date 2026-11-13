// Réglages et état d'interface partagés, enregistrés dans la base via le cœur Rust.
import { api, type AchievementRow } from "./library";
import { send } from "./sync";

export const settings = $state({
  /** Floute nom et description des succès cachés non obtenus (activé par défaut). */
  antiSpoiler: true,
  /** Vue de l'espace de travail : tableau libre (grille) ou liste. */
  workspaceView: "board" as "board" | "list",
  /** Panneau de droite (liste complète des succès) ouvert. */
  panelOpen: true,
  /** Panneau de gauche (liste des jeux) déplié ; replié, il ne montre que les jaquettes. */
  sidebarOpen: true,
  /** Opacité de la fenêtre compacte (0,3 à 1). */
  compactOpacity: 0.95,
});

export const clampOpacity = (v: number) => (Number.isFinite(v) ? Math.min(1, Math.max(0.3, v)) : 0.95);

/** État éphémère d'interface. */
export const ui = $state({
  /** Un glisser depuis le panneau est en cours (pour montrer les zones d'arrivée). */
  draggingFromPanel: false,
  /** Succès en cours de glisser depuis le panneau (le premier donne la taille de l'aperçu). */
  draggingIds: [] as number[],
  /** Assistant de re-lock : ouvert ou non, pour un succès ou (null) le jeu entier. */
  relock: { open: false, target: null as AchievementRow | null },
});

export function openRelock(target: AchievementRow | null) {
  // Champs modifiés un par un : remplacer l'objet réveillerait les effets qui lisent `ui.relock`
  ui.relock.target = target;
  ui.relock.open = true;
}

export async function loadSettings() {
  const [spoiler, view, panel, opacity, sidebar] = await Promise.all(
    (["anti_spoiler", "workspace_view", "panel_open", "compact_opacity", "sidebar_open"] as const).map((k) =>
      api.getSetting(k).catch(() => null),
    ),
  );
  if (opacity) settings.compactOpacity = clampOpacity(Number(opacity));
  settings.antiSpoiler = spoiler !== "0";
  // « columns » (ancienne vue Colonnes, retirée) devient le tableau
  settings.workspaceView = view === "list" ? "list" : "board";
  settings.panelOpen = panel !== "0";
  settings.sidebarOpen = sidebar !== "0";
}

export async function setAntiSpoiler(on: boolean) {
  settings.antiSpoiler = on;
  await api.setSetting("anti_spoiler", on ? "1" : "0");
}

export async function setWorkspaceView(view: "board" | "list") {
  settings.workspaceView = view;
  await api.setSetting("workspace_view", view);
}

export async function setCompactOpacity(value: number) {
  settings.compactOpacity = clampOpacity(value);
  send("compact-opacity", { value: settings.compactOpacity });
  await api.setSetting("compact_opacity", String(settings.compactOpacity));
}

export async function setPanelOpen(open: boolean) {
  settings.panelOpen = open;
  await api.setSetting("panel_open", open ? "1" : "0");
}

export async function setSidebarOpen(open: boolean) {
  settings.sidebarOpen = open;
  await api.setSetting("sidebar_open", open ? "1" : "0");
}
