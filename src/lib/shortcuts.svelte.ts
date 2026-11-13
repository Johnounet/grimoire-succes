// Raccourcis clavier globaux du mode compact : ils marchent même quand le jeu est au premier plan.
//
// - Haut / Bas : élément précédent / suivant de la fenêtre compacte (compteurs, chronos, vidéos, listes) ;
// - Gauche / Droite : action sur l'élément sélectionné (détail dans `Compact.svelte`).
//
// Ils ne sont enregistrés auprès du système que pendant que la fenêtre compacte est ouverte : c'est elle
// qui les enregistre et les libère. Le reste du temps, les touches restent libres pour les autres logiciels.
// La fenêtre principale ne fait que les régler (Réglages) et prévient la fenêtre compacte
// (`shortcuts-changed`), qui lui renvoie l'état et les refus du système (`shortcuts-status`).
import { register, unregisterAll } from "@tauri-apps/plugin-global-shortcut";
import { api } from "./library";
import { send } from "./sync";

export type ShortcutAction = "select_prev" | "select_next" | "act_left" | "act_right";

export const shortcutActions: { action: ShortcutAction; label: string; hint: string }[] = [
  { action: "select_prev", label: "Élément précédent", hint: "" },
  { action: "select_next", label: "Élément suivant", hint: "" },
  { action: "act_left", label: "Action gauche", hint: "−1 · chrono à zéro (à l'arrêt) · vidéo −10 s · décocher" },
  { action: "act_right", label: "Action droite", hint: "+1 · lancer / arrêter le chrono · lecture / pause · cocher" },
];

/** Touches par défaut : peu utilisées par les jeux. Format des raccourcis Tauri (`KeyboardEvent.code`). */
export const DEFAULT_SHORTCUTS: Record<ShortcutAction, string | null> = {
  select_prev: "Ctrl+Alt+ArrowUp",
  select_next: "Ctrl+Alt+ArrowDown",
  act_left: "Ctrl+Alt+ArrowLeft",
  act_right: "Ctrl+Alt+ArrowRight",
};

/** Actions d'avant la refonte du mode compact → nouvelles actions, sur les mêmes flèches par défaut. */
const LEGACY: Record<string, ShortcutAction> = {
  counter_inc: "select_prev",
  counter_dec: "select_next",
  timer_reset: "act_left",
  timer_toggle: "act_right",
};

const isAction = (a: string): a is ShortcutAction => a in DEFAULT_SHORTCUTS;

export const shortcuts = $state({
  keys: { ...DEFAULT_SHORTCUTS },
  /** Petit son de confirmation. */
  sound: true,
  /** Raccourcis refusés par le système (déjà pris par une autre application…). */
  errors: {} as Partial<Record<ShortcutAction, string>>,
  /** Fenêtre compacte ouverte : les raccourcis sont enregistrés. */
  live: false,
});

export async function loadShortcuts() {
  const [raw, sound] = await Promise.all([api.getSetting("shortcuts"), api.getSetting("shortcut_sound")]).catch(() => [null, null]);
  const keys = { ...DEFAULT_SHORTCUTS };
  try {
    const saved = raw ? (JSON.parse(raw) as Record<string, string | null>) : {};
    for (const [name, accel] of Object.entries(saved)) {
      const action = LEGACY[name] ?? name;
      if (isAction(action)) keys[action] = accel ?? null;
    }
  } catch {
    /* réglage illisible : touches par défaut */
  }
  shortcuts.keys = keys;
  shortcuts.sound = sound !== "0";
}

export async function setShortcut(action: ShortcutAction, accel: string | null) {
  shortcuts.keys[action] = accel;
  await api.setSetting("shortcuts", JSON.stringify(shortcuts.keys));
  send("shortcuts-changed", {});
}

export async function setShortcutSound(on: boolean) {
  shortcuts.sound = on;
  await api.setSetting("shortcut_sound", on ? "1" : "0");
  send("shortcuts-changed", {});
}

/** Fenêtre compacte : (ré)enregistre tous les raccourcis auprès du système. */
export async function applyShortcuts(handler: (action: ShortcutAction) => void) {
  const errors: Partial<Record<ShortcutAction, string>> = {};
  try {
    await unregisterAll();
  } catch {
    /* hors de Tauri */
  }
  const seen = new Map<string, ShortcutAction>();
  for (const { action } of shortcutActions) {
    const accel = shortcuts.keys[action];
    if (!accel) continue;
    const key = accel.toLowerCase();
    if (seen.has(key)) {
      errors[action] = "Même touche qu'une autre action.";
      continue;
    }
    seen.set(key, action);
    try {
      await register(accel, (e) => {
        if (e.state === "Pressed") handler(action);
      });
    } catch (e) {
      errors[action] = `Refusé par le système : ${e}`;
    }
  }
  shortcuts.errors = errors;
  shortcuts.live = true;
  send("shortcuts-status", { errors, live: true });
}

/** Fenêtre compacte fermée : les touches sont rendues aux autres logiciels. */
export async function releaseShortcuts() {
  try {
    await unregisterAll();
  } catch {
    /* hors de Tauri */
  }
  shortcuts.errors = {};
  shortcuts.live = false;
  send("shortcuts-status", { errors: {}, live: false });
}

// ----- Affichage et saisie des touches -----

const KEY_NAMES: Record<string, string> = {
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Space: "Espace",
  Enter: "Entrée",
  Backspace: "Retour arrière",
  Delete: "Suppr",
  Insert: "Inser",
  Home: "Début",
  End: "Fin",
  PageUp: "Page préc.",
  PageDown: "Page suiv.",
  NumpadAdd: "Pavé +",
  NumpadSubtract: "Pavé −",
  NumpadMultiply: "Pavé ×",
  NumpadDivide: "Pavé ÷",
  NumpadEnter: "Pavé Entrée",
  Minus: "-",
  Equal: "=",
};

/** « Ctrl+Alt+ArrowUp » → « Ctrl + Alt + ↑ ». */
export function formatAccel(accel: string | null): string {
  if (!accel) return "aucun";
  return accel
    .split("+")
    .map((p) => {
      if (p === "Super") return "Win";
      if (KEY_NAMES[p]) return KEY_NAMES[p];
      if (/^Key[A-Z]$/.test(p)) return p.slice(3);
      if (/^Digit\d$/.test(p)) return p.slice(5);
      if (/^Numpad\d$/.test(p)) return `Pavé ${p.slice(6)}`;
      return p;
    })
    .join(" + ");
}

const MODIFIER_CODES = /^(Control|Alt|Shift|Meta|OS)(Left|Right)?$/;

/**
 * Touche pressée → raccourci au format Tauri, ou null tant que la combinaison est incomplète.
 * Un modificateur est exigé (sauf touches F1–F24), pour ne pas voler une touche au jeu.
 */
export function accelFromEvent(e: KeyboardEvent): string | null {
  if (MODIFIER_CODES.test(e.code) || !e.code) return null;
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
  if (mods.length === 0 && !/^F\d{1,2}$/.test(e.code)) return null;
  return [...mods, e.code].join("+");
}
