// Gestes sur les compteurs, chronomètres et listes, partagés par les blocs, le mode compact
// et les raccourcis clavier globaux (logique pure : modifie `data` sur place).
import type { CounterData, ListData, TimerData } from "./types";

/** +1 / −1 (multiplié par le pas), jamais sous zéro. */
export function counterAdd(d: CounterData, sign: 1 | -1) {
  const step = Math.max(1, Math.abs(d.step || 1));
  d.value = Math.max(0, d.value + sign * step);
}

export const timerRunning = (d: TimerData) => d.running_since != null;

export function timerElapsed(d: TimerData, now: number): number {
  return d.elapsed_ms + (d.running_since != null ? Math.max(0, now - d.running_since) : 0);
}

/** Une nouvelle course repart de zéro. */
export function timerStart(d: TimerData, now: number) {
  d.elapsed_ms = 0;
  d.running_since = now;
}

/** Arrête la course. Renvoie true si c'est un nouveau meilleur temps. */
export function timerStop(d: TimerData, now: number): boolean {
  d.elapsed_ms = timerElapsed(d, now);
  d.running_since = null;
  if (d.elapsed_ms > 0 && (d.best_ms == null || d.elapsed_ms < d.best_ms)) {
    d.best_ms = d.elapsed_ms;
    return true;
  }
  return false;
}

/** Démarre ou arrête. Renvoie ce qui s'est passé. */
export function timerToggle(d: TimerData, now: number): "started" | "stopped" | "record" {
  if (timerRunning(d)) return timerStop(d, now) ? "record" : "stopped";
  timerStart(d, now);
  return "started";
}

export function timerReset(d: TimerData) {
  d.elapsed_ms = 0;
  d.running_since = null;
}

/** Coche le premier élément non coché. Renvoie son texte, ou null si tout est coché. */
export function listCheckNext(d: ListData): string | null {
  const it = d.items.find((i) => !i.checked);
  if (!it) return null;
  it.checked = true;
  return it.text;
}

/** Décoche le dernier élément coché (dans l'ordre de la liste). Renvoie son texte, ou null. */
export function listUncheckLast(d: ListData): string | null {
  const it = [...d.items].reverse().find((i) => i.checked);
  if (!it) return null;
  it.checked = false;
  return it.text;
}
