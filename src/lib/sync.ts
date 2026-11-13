// Événements entre la fenêtre principale et la fenêtre compacte (événements Tauri).
// Chaque fenêtre garde son propre état ; ces messages le tiennent à jour.
// Un événement porte la fenêtre d'origine : on n'écoute pas ses propres messages.
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Block } from "./blocks/types";
import type { GameSummary } from "./library";

// Lu au chargement du module : main.ts n'importe l'interface qu'après avoir installé
// le simulateur (interface simulée), qui fournit lui aussi un nom de fenêtre.
export const windowLabel: string = (() => {
  try {
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
})();

export interface SyncEvents {
  /** Un compteur, chrono ou liste a changé (clic, raccourci). */
  "block-changed": { block: Block };
  /** Changement de structure (cartes, blocs ajoutés ou supprimés, coche) : tout recharger. */
  "data-changed": { appid: number };
  /** Un succès a été coché ou décoché depuis la fenêtre compacte. */
  "achievement-changed": { id: number; unlocked: boolean; game: GameSummary };
  /** Réglage des raccourcis changé (fenêtre principale) : la fenêtre compacte les réenregistre. */
  "shortcuts-changed": Record<string, never>;
  /** Raccourcis enregistrés ou libérés par la fenêtre compacte, et refus du système. */
  "shortcuts-status": { errors: Partial<Record<string, string>>; live: boolean };
  /** Envoyé par le cœur Rust (`watch.rs`) : Steam a réécrit la progression d'un jeu de la bibliothèque. */
  "steam-progress": { game: GameSummary; unlocked: { id: number; name: string }[] };
  /** Une vidéo démarre dans une fenêtre : l'autre met en pause sa copie du même bloc. */
  "video-playing": { blockId: number };
  /** Jeu ouvert dans la fenêtre principale. */
  "compact-game": { appid: number };
  /** La fenêtre compacte vient de s'ouvrir et demande le jeu ouvert. */
  "compact-ready": Record<string, never>;
  "compact-opacity": { value: number };
  /** Réglage des animations changé. */
  "motion-changed": { pref: "system" | "on" | "off" };
  /** Apparence ou thème changé. */
  "theme-changed": { mode: "system" | "dark" | "light"; dark: string; light: string };
  /** Une sauvegarde a été restaurée : tout recharger. */
  "data-restored": Record<string, never>;
  /** Réglage du carillon à la coche changé. */
  "sound-changed": { check: boolean };
}

type Name = keyof SyncEvents;

export function send<K extends Name>(name: K, payload: SyncEvents[K]) {
  emit(name, { ...payload, source: windowLabel }).catch(() => {
    /* hors de Tauri sans simulateur d'événements */
  });
}

/** Écoute les messages des autres fenêtres. Renvoie la fonction d'arrêt (pour un $effect). */
export function on<K extends Name>(name: K, cb: (payload: SyncEvents[K]) => void): () => void {
  let stop: (() => void) | null = null;
  let dead = false;
  listen<SyncEvents[K] & { source: string }>(name, (e) => {
    if (e.payload.source !== windowLabel) cb(e.payload);
  })
    .then((u) => (dead ? u() : (stop = u)))
    .catch(() => {});
  return () => {
    dead = true;
    stop?.();
  };
}
