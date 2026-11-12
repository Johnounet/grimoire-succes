// Compteur et chrono « actifs » et raccourcis clavier globaux, qui agissent sur eux même quand
// le jeu est au premier plan.
//
// Bloc visé par un raccourci : celui choisi avec l'icône ◎ ; à défaut, le premier compteur (ou chrono)
// des cartes épinglées au mode compact. Une carte non épinglée n'est jamais visée par défaut : les
// raccourcis n'agissent que sur ce que montre la fenêtre compacte. Désépingler une carte retire aussi
// le ◎ de ses blocs (`releaseActive`).
//
// Les raccourcis sont enregistrés par la fenêtre principale seulement. Le bloc actif peut appartenir
// à un autre jeu que celui affiché : il est alors lu et enregistré directement en base.
import { invoke } from "@tauri-apps/api/core";
import { register, unregisterAll } from "@tauri-apps/plugin-global-shortcut";
import { counterAdd, timerReset, timerToggle } from "./blocks/ops";
import { blocksOf, findBlock, saveBlock, showToast } from "./blocks/store.svelte";
import { formatDuration, type Block, type CounterData, type TimerData } from "./blocks/types";
import { api, type WorkspaceItem } from "./library";
import { beep } from "./sound.svelte";
import { send } from "./sync";

export type ShortcutAction = "counter_inc" | "counter_dec" | "timer_toggle" | "timer_reset";

export const shortcutActions: { action: ShortcutAction; label: string }[] = [
  { action: "counter_inc", label: "Compteur actif : +1" },
  { action: "counter_dec", label: "Compteur actif : −1" },
  { action: "timer_toggle", label: "Chrono actif : démarrer / arrêter" },
  { action: "timer_reset", label: "Chrono actif : remettre à zéro" },
];

/** Touches par défaut : peu utilisées par les jeux. Format des raccourcis Tauri (`KeyboardEvent.code`). */
export const DEFAULT_SHORTCUTS: Record<ShortcutAction, string | null> = {
  counter_inc: "Ctrl+Alt+ArrowUp",
  counter_dec: "Ctrl+Alt+ArrowDown",
  timer_toggle: "Ctrl+Alt+ArrowRight",
  timer_reset: "Ctrl+Alt+ArrowLeft",
};

export const active = $state({
  counter: null as number | null,
  timer: null as number | null,
});

/** Espace de travail du jeu ouvert (pour choisir un bloc par défaut). Tenu à jour par la vue du jeu. */
export const shortcutContext = $state({ workspace: [] as WorkspaceItem[] });

type Kind = "counter" | "timer";

/** Bloc visé par les raccourcis pour ce type, et s'il a été choisi (◎) ou pris par défaut. */
export function target(kind: Kind): { id: number; chosen: boolean } | null {
  const chosen = active[kind];
  if (chosen != null) return { id: chosen, chosen: true };
  for (const c of shortcutContext.workspace) {
    if (!c.pinned) continue;
    const b = blocksOf(c.achievement_id).find((x) => x.kind === kind);
    if (b) return { id: b.id, chosen: false };
  }
  return null;
}

export const shortcuts = $state({
  keys: { ...DEFAULT_SHORTCUTS },
  /** Petit son de confirmation. */
  sound: true,
  /** Raccourcis refusés par le système (déjà pris par une autre application…). */
  errors: {} as Partial<Record<ShortcutAction, string>>,
});

const num = (v: string | null) => (v && Number.isFinite(Number(v)) ? Number(v) : null);

export async function loadActive() {
  const [c, t] = await Promise.all([api.getSetting("active_counter"), api.getSetting("active_timer")]).catch(() => [null, null]);
  active.counter = num(c);
  active.timer = num(t);
}

/** Choisit (ou retire, si c'est déjà lui) le compteur ou chrono actif. */
export async function toggleActive(kind: Kind, id: number) {
  active[kind] = active[kind] === id ? null : id;
  send("active-changed", { counter: active.counter, timer: active.timer });
  await api.setSetting(kind === "counter" ? "active_counter" : "active_timer", active[kind] == null ? "" : String(active[kind]));
}

/** Carte retirée du mode compact : ses blocs cessent d'être visés par les raccourcis. */
export async function releaseActive(achievementId: number) {
  for (const kind of ["counter", "timer"] as const) {
    const id = active[kind];
    if (id != null && findBlock(id)?.achievement_id === achievementId) await toggleActive(kind, id);
  }
}

export async function loadShortcuts() {
  const [raw, sound] = await Promise.all([api.getSetting("shortcuts"), api.getSetting("shortcut_sound")]).catch(() => [null, null]);
  try {
    const saved = raw ? (JSON.parse(raw) as Partial<Record<ShortcutAction, string | null>>) : {};
    for (const { action } of shortcutActions) if (action in saved) shortcuts.keys[action] = saved[action] ?? null;
  } catch {
    /* réglage illisible : touches par défaut */
  }
  shortcuts.sound = sound !== "0";
}

export async function setShortcut(action: ShortcutAction, accel: string | null) {
  shortcuts.keys[action] = accel;
  await api.setSetting("shortcuts", JSON.stringify(shortcuts.keys));
  await applyShortcuts();
}

export async function setShortcutSound(on: boolean) {
  shortcuts.sound = on;
  await api.setSetting("shortcut_sound", on ? "1" : "0");
}

/** (Ré)enregistre tous les raccourcis auprès du système. */
export async function applyShortcuts() {
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
        if (e.state === "Pressed") run(action);
      });
    } catch (e) {
      errors[action] = `Refusé par le système : ${e}`;
    }
  }
  shortcuts.errors = errors;
}

/** Applique l'action sur le bloc actif, où qu'il soit. */
export async function run(action: ShortcutAction) {
  const kind: Kind = action.startsWith("counter") ? "counter" : "timer";
  const t = target(kind);
  if (!t)
    return feedback(
      false,
      kind === "counter"
        ? "Aucun compteur visé : épinglez au mode compact une carte qui en contient un, ou choisissez-le avec ◎"
        : "Aucun chrono visé : épinglez au mode compact une carte qui en contient un, ou choisissez-le avec ◎",
    );
  const id = t.id;

  let block = findBlock(id);
  let label = "";
  if (!block) {
    const info = await invoke<{ block: Block; achievement_name: string } | null>("block_get", { id }).catch(() => null);
    if (!info || info.block.kind !== kind) {
      // Bloc supprimé entre-temps
      await toggleActive(kind, id);
      return feedback(false, kind === "counter" ? "Le compteur actif n'existe plus" : "Le chrono actif n'existe plus");
    }
    block = info.block;
    label = info.achievement_name;
  }

  const text = apply(action, block, Date.now());
  const name = (block.data as CounterData | TimerData).label || label;
  if (findBlock(id)) {
    saveBlock(block);
  } else {
    // Autre jeu : enregistré tout de suite, et annoncé à la fenêtre compacte
    await invoke("block_update", { id, data: block.data }).catch(() => {});
    send("block-changed", { block });
  }
  feedback(true, name ? `${name} : ${text}` : text);
}

function apply(action: ShortcutAction, block: Block, now: number): string {
  if (block.kind === "counter") {
    const d = block.data as CounterData;
    counterAdd(d, action === "counter_inc" ? 1 : -1);
    return d.goal != null ? `${d.value} / ${d.goal}` : String(d.value);
  }
  const d = block.data as TimerData;
  if (action === "timer_reset") {
    timerReset(d);
    return "remis à zéro";
  }
  const what = timerToggle(d, now);
  if (what === "started") return "chrono lancé";
  return `${formatDuration(d.elapsed_ms)}${what === "record" ? " · nouveau record !" : ""}`;
}

function feedback(ok: boolean, text: string) {
  showToast(text);
  send("shortcut-fired", { text, ok });
  if (shortcuts.sound) beep(ok);
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
