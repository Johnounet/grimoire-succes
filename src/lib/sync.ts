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
  /** Un compteur ou chrono a changé (clic, raccourci). */
  "block-changed": { block: Block };
  /** Changement de structure (cartes, blocs ajoutés ou supprimés, coche) : tout recharger. */
  "data-changed": { appid: number };
  /** Un succès a été coché ou décoché depuis la fenêtre compacte. */
  "achievement-changed": { id: number; unlocked: boolean; game: GameSummary };
  /** Le compteur ou chrono actif a changé. */
  "active-changed": { counter: number | null; timer: number | null };
  /** Un raccourci global a agi (retour visuel). */
  "shortcut-fired": { text: string; ok: boolean };
  /** Jeu ouvert dans la fenêtre principale. */
  "compact-game": { appid: number };
  /** La fenêtre compacte vient de s'ouvrir et demande le jeu ouvert. */
  "compact-ready": Record<string, never>;
  "compact-opacity": { value: number };
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
