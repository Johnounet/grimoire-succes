// Types des blocs de guide (miroir de la table `blocks`, champ `data` en JSON).
import type { CardOwner } from "../library";

export type BlockKind = "video" | "image" | "text" | "list" | "counter" | "timer";

export interface VideoData {
  url: string;
  /** Repères : temps en secondes et libellé (« 3:42 → le coffre caché »). */
  markers: { time: number; label: string }[];
  /** Titre YouTube, relevé par le lecteur à l'affichage (absent tant que la vidéo n'a pas été chargée). */
  title?: string;
}
export interface ImageData {
  /** Chemin relatif au dossier images/ (ex. blocks/123.png). */
  file: string | null;
  caption: string;
}
export interface TextData {
  markdown: string;
}
export interface ListData {
  items: { text: string; checked: boolean }[];
}
export interface CounterData {
  label: string;
  value: number;
  goal: number | null;
  step: number;
}
export interface TimerData {
  label: string;
  /** Temps cible facultatif (ms). */
  target_ms: number | null;
  best_ms: number | null;
  /** Temps accumulé hors course en cours (ms). */
  elapsed_ms: number;
  /** Horodatage (ms) du démarrage en cours, ou null à l'arrêt. Permet de retrouver un chrono lancé après redémarrage. */
  running_since: number | null;
}

export interface DataByKind {
  video: VideoData;
  image: ImageData;
  text: TextData;
  list: ListData;
  counter: CounterData;
  timer: TimerData;
}

/** `achievement_id` ou `card_id` (carte libre) : voir `cardKey`. */
export interface Block<K extends BlockKind = BlockKind> extends CardOwner {
  id: number;
  kind: K;
  position: number;
  /** `collapsed` : bloc replié dans sa carte (fenêtre principale seulement), commun à tous les types. */
  data: DataByKind[K] & { collapsed?: boolean };
}

export const blockKinds: { kind: BlockKind; label: string; icon: string; hint: string }[] = [
  { kind: "video", label: "Vidéo YouTube", icon: "▶", hint: "Lecteur intégré et repères de temps" },
  { kind: "image", label: "Image", icon: "▣", hint: "Fichier, glisser-déposer ou Ctrl+V" },
  { kind: "text", label: "Texte", icon: "¶", hint: "Notes avec gras, titres, listes, liens" },
  { kind: "list", label: "Liste", icon: "☑", hint: "Cases à cocher : collectibles, étapes" },
  { kind: "counter", label: "Compteur", icon: "#", hint: "Valeur / objectif, +1 / −1" },
  { kind: "timer", label: "Chronomètre", icon: "⏱", hint: "Temps cible et meilleur temps" },
];

export const kindInfo = (k: BlockKind) => blockKinds.find((b) => b.kind === k)!;

export function defaultData<K extends BlockKind>(kind: K): DataByKind[K] {
  const d: DataByKind = {
    video: { url: "", markers: [] },
    image: { file: null, caption: "" },
    text: { markdown: "" },
    list: { items: [] },
    counter: { label: "", value: 0, goal: null, step: 1 },
    timer: { label: "", target_ms: null, best_ms: null, elapsed_ms: 0, running_since: null },
  };
  return d[kind];
}

/** Durée en ms → « 4:05.32 » ou « 1:02:03.40 ». */
export function formatDuration(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 10));
  const cs = total % 100;
  const s = Math.floor(total / 100) % 60;
  const m = Math.floor(total / 6000) % 60;
  const h = Math.floor(total / 360000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}.${pad(cs)}` : `${m}:${pad(s)}.${pad(cs)}`;
}

/** « 3:42 », « 1:02:03 », « 42 » ou « 4:05.5 » → secondes. null si illisible. */
export function parseClock(input: string): number | null {
  const s = input.trim().replace(",", ".");
  if (!/^\d+(:\d{1,2}){0,2}(\.\d+)?$/.test(s)) return null;
  const parts = s.split(":").map(Number);
  return parts.reduce((acc, p) => acc * 60 + p, 0);
}

/** Secondes → « 3:42 » ou « 1:02:03 » (sans centièmes). */
export function formatClock(sec: number): string {
  const t = Math.max(0, Math.floor(sec));
  const pad = (n: number) => String(n).padStart(2, "0");
  const h = Math.floor(t / 3600);
  const m = Math.floor(t / 60) % 60;
  return h > 0 ? `${h}:${pad(m)}:${pad(t % 60)}` : `${m}:${pad(t % 60)}`;
}
