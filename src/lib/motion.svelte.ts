// Animations : réglage « selon Windows / activées / désactivées » et petites transitions communes.
// Les animations CSS sont coupées par l'attribut `data-reduced-motion` sur <html> (voir app.css).
// Celles lancées en JavaScript (transitions Svelte, `flip`, Web Animations) passent
// par `dur()`, qui renvoie 0 quand les animations sont réduites.
import { untrack } from "svelte";
import { cubicOut } from "svelte/easing";
import type { TransitionConfig } from "svelte/transition";
import { api } from "./library";
import { send } from "./sync";

export type MotionPref = "system" | "on" | "off";

const query = typeof window !== "undefined" ? window.matchMedia("(prefers-reduced-motion: reduce)") : null;

export const motion = $state({
  pref: "system" as MotionPref,
  /** « Réduire les animations » est actif dans Windows. */
  system: query?.matches ?? false,
});
query?.addEventListener("change", (e) => (motion.system = e.matches));

/** Les animations sont-elles réduites (réglage du logiciel, ou de Windows si « selon Windows ») ? */
export const reduced = () => motion.pref === "off" || (motion.pref === "system" && motion.system);

/** Durées communes (ms) : tout le logiciel bouge au même rythme. */
export const SPEED = { fast: 120, base: 180, slow: 300 } as const;

/** Durée à utiliser : 0 si les animations sont réduites. */
export const dur = (ms: number) => (reduced() ? 0 : ms);

export const parsePref = (v: string | null): MotionPref => (v === "on" || v === "off" ? v : "system");

export async function loadMotion() {
  motion.pref = parsePref(await api.getSetting("animations").catch(() => null));
}

export async function setMotionPref(pref: MotionPref) {
  motion.pref = pref;
  send("motion-changed", { pref });
  await api.setSetting("animations", pref);
}

/** À appeler dans un $effect de chaque fenêtre : tient à jour l'attribut lu par le CSS. */
export function applyMotionAttribute() {
  document.documentElement.toggleAttribute("data-reduced-motion", reduced());
}

// ----- Transitions Svelte -----

/** Fond d'une fenêtre modale : fondu. */
export function backdrop(_node: Element): TransitionConfig {
  return { duration: dur(SPEED.base), css: (t) => `opacity: ${t}` };
}

/** Fenêtre modale ou menu : léger zoom avec fondu. `y` : petit glissement vertical (px). */
export function pop(_node: Element, { y = 0, start = 0.96 }: { y?: number; start?: number } = {}): TransitionConfig {
  return {
    duration: dur(SPEED.base),
    easing: cubicOut,
    css: (t, u) => `opacity: ${t}; transform: translateY(${u * y}px) scale(${start + (1 - start) * t})`,
  };
}

// ----- Animations ponctuelles (Web Animations) -----
// `appear` et `arrive` lisent le réglage sans dépendance (`untrack`) : appelées depuis des
// attachements, elles ne doivent pas se rejouer quand le réglage change.

/** Apparition douce d'une zone (changement de jeu, accueil). */
export function appear(el: Element, y = 6) {
  if (untrack(reduced)) return;
  el.animate(
    [
      { opacity: 0, transform: `translateY(${y}px)` },
      { opacity: 1, transform: "none" },
    ],
    { duration: SPEED.slow, easing: "cubic-bezier(0.2, 0.8, 0.2, 1)" },
  );
}

