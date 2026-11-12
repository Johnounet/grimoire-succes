// Petits sons synthétisés (aucun fichier audio) : bip des raccourcis, carillon d'un succès coché.
import { api } from "./library";
import { send } from "./sync";

export const sound = $state({
  /** Carillon discret quand on coche un succès (désactivé par défaut). */
  check: false,
});

export async function loadSound() {
  sound.check = (await api.getSetting("check_sound").catch(() => null)) === "1";
}

export async function setCheckSound(on: boolean) {
  sound.check = on;
  send("sound-changed", { check: on });
  await api.setSetting("check_sound", on ? "1" : "0");
}

let audio: AudioContext | null = null;

/** Une note : `freq` Hz, départ dans `at` s, volume `peak`, durée `len` s. */
function note(ctx: AudioContext, freq: number, at: number, peak: number, len: number, type: OscillatorType = "sine") {
  const t = ctx.currentTime + at;
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  osc.type = type;
  osc.frequency.value = freq;
  gain.gain.setValueAtTime(0.0001, t);
  gain.gain.exponentialRampToValueAtTime(peak, t + 0.01);
  gain.gain.exponentialRampToValueAtTime(0.0001, t + len);
  osc.connect(gain).connect(ctx.destination);
  osc.start(t);
  osc.stop(t + len + 0.01);
}

function play(fn: (ctx: AudioContext) => void) {
  try {
    audio ??= new AudioContext();
    fn(audio);
  } catch {
    /* pas de son disponible */
  }
}

/** Bip court de confirmation (raccourcis globaux) : aigu si réussi, grave sinon. */
export function beep(ok: boolean) {
  play((ctx) => note(ctx, ok ? 880 : 220, 0, 0.12, 0.12));
}

/** Carillon d'un succès coché : deux notes douces qui montent (mi, si). */
export function chime() {
  if (!sound.check) return;
  play((ctx) => {
    note(ctx, 659.25, 0, 0.07, 0.35);
    note(ctx, 987.77, 0.08, 0.06, 0.5);
  });
}
