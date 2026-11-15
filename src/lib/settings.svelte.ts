// Réglages et état d'interface partagés, enregistrés dans la base via le cœur Rust.
import type { StateFilter } from "./filters";
import { api, type AchievementRow } from "./library";
import { send } from "./sync";

export const settings = $state({
  /** Floute nom et description des succès cachés non obtenus (activé par défaut). */
  antiSpoiler: true,
  /** Filtre d'état de la vue des succès, retenu (« À obtenir » la première fois). */
  stateFilter: "locked" as StateFilter,
  /** Panneau de gauche (liste des jeux) déplié ; replié, il ne montre que les jaquettes. */
  sidebarOpen: true,
  /** Opacité de la fenêtre compacte (0,3 à 1). */
  compactOpacity: 0.95,
  /** Largeur minimale des colonnes de cartes de la fenêtre compacte (px). */
  compactColumnWidth: 420,
});

export const clampOpacity = (v: number) => (Number.isFinite(v) ? Math.min(1, Math.max(0.3, v)) : 0.95);
export const COLUMN_WIDTH = { min: 280, max: 960, step: 20 };
export const clampColumnWidth = (v: number) =>
  Number.isFinite(v) ? Math.min(COLUMN_WIDTH.max, Math.max(COLUMN_WIDTH.min, Math.round(v))) : 420;

/** État éphémère d'interface. */
export const ui = $state({
  /** Assistant de re-lock : ouvert ou non, pour un succès ou (null) le jeu entier. */
  relock: { open: false, target: null as AchievementRow | null },
});

export function openRelock(target: AchievementRow | null) {
  // Champs modifiés un par un : remplacer l'objet réveillerait les effets qui lisent `ui.relock`
  ui.relock.target = target;
  ui.relock.open = true;
}

export async function loadSettings() {
  const [spoiler, state, opacity, sidebar, columnWidth] = await Promise.all(
    (["anti_spoiler", "state_filter", "compact_opacity", "sidebar_open", "compact_column_width"] as const).map((k) =>
      api.getSetting(k).catch(() => null),
    ),
  );
  if (opacity) settings.compactOpacity = clampOpacity(Number(opacity));
  if (columnWidth) settings.compactColumnWidth = clampColumnWidth(Number(columnWidth));
  settings.antiSpoiler = spoiler !== "0";
  if (state === "all" || state === "unlocked" || state === "locked") settings.stateFilter = state;
  settings.sidebarOpen = sidebar !== "0";
}

export async function setAntiSpoiler(on: boolean) {
  settings.antiSpoiler = on;
  await api.setSetting("anti_spoiler", on ? "1" : "0");
}

export async function setStateFilter(state: StateFilter) {
  settings.stateFilter = state;
  await api.setSetting("state_filter", state);
}

export async function setCompactOpacity(value: number) {
  settings.compactOpacity = clampOpacity(value);
  send("compact-opacity", { value: settings.compactOpacity });
  await api.setSetting("compact_opacity", String(settings.compactOpacity));
}

/** Largeur des colonnes de la fenêtre compacte : `save` à false pendant le glisser du curseur
 *  (affichage seulement), puis true au lâcher (un seul enregistrement). */
export async function setCompactColumnWidth(value: number, save = true) {
  settings.compactColumnWidth = clampColumnWidth(value);
  if (save) await api.setSetting("compact_column_width", String(settings.compactColumnWidth));
}

export async function setSidebarOpen(open: boolean) {
  settings.sidebarOpen = open;
  await api.setSetting("sidebar_open", open ? "1" : "0");
}
