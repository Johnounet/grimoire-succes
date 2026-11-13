// État des blocs du jeu ouvert, et leur enregistrement automatique.
//
// - Une modification s'affiche tout de suite et part en base après un court délai
//   (les frappes rapprochées sont regroupées) ; `flushBlocks()` force l'envoi immédiat.
// - Une suppression peut être annulée quelques secondes (toast « Annuler »).
// - Compteurs, chronos et listes sont aussi affichés dans la fenêtre compacte : chaque changement
//   lui est annoncé (`block-changed`), et ceux qu'elle fait arrivent ici de la même façon.
import { invoke } from "@tauri-apps/api/core";
import { cardKey, ownerOf, type CardKey } from "../library";
import { on, send } from "../sync";
import { defaultData, kindInfo, type Block, type BlockKind } from "./types";

const SAVE_DELAY_MS = 400;
const UNDO_MS = 8000;

export const blocks = $state({
  appid: 0,
  /** Blocs par carte (`cardKey` : succès ou carte libre), dans l'ordre. */
  byCard: {} as Record<CardKey, Block[]>,
  error: "",
});

export const toast = $state({
  message: "",
  undo: null as null | (() => void),
});

const pending = new Map<number, { timer: ReturnType<typeof setTimeout>; send: () => Promise<void> }>();

function fail(e: unknown) {
  blocks.error = `Enregistrement d'un bloc impossible : ${e}`;
}

export async function loadBlocks(appid: number) {
  await flushBlocks();
  blocks.appid = appid;
  blocks.byCard = {};
  blocks.error = "";
  try {
    const list = await invoke<Block[]>("blocks_list", { appid });
    if (blocks.appid !== appid) return;
    const grouped: Record<CardKey, Block[]> = {};
    for (const b of list) (grouped[cardKey(b)] ??= []).push(b);
    blocks.byCard = grouped;
  } catch (e) {
    blocks.error = `Chargement des blocs impossible : ${e}`;
  }
}

export const blocksOf = (key: CardKey): Block[] => blocks.byCard[key] ?? [];

/** Bloc du jeu ouvert, ou undefined s'il n'est pas chargé ici. */
export function findBlock(id: number): Block | undefined {
  for (const list of Object.values(blocks.byCard)) {
    const b = list.find((x) => x.id === id);
    if (b) return b;
  }
  return undefined;
}

/** Changement de structure : la fenêtre compacte se recharge. */
const structureChanged = () => send("data-changed", { appid: blocks.appid });

/** Reçoit les compteurs et chronos modifiés dans l'autre fenêtre. À appeler une fois par fenêtre. */
export function listenBlockChanges(): () => void {
  return on("block-changed", ({ block }) => {
    const b = findBlock(block.id);
    if (b) b.data = block.data;
  });
}

/** Nouveau bloc sur la carte `key` (succès ou carte libre). */
export async function createBlock<K extends BlockKind>(
  key: CardKey,
  kind: K,
  data = defaultData(kind),
  position?: number,
): Promise<Block | null> {
  try {
    const o = ownerOf(key);
    const b = await invoke<Block>("block_create", {
      achievementId: o.achievement_id,
      cardId: o.card_id,
      kind,
      data,
      position: position ?? null,
    });
    // Pas de `??=` ici : sur un état réactif il renverrait le tableau brut, pas sa version suivie
    if (!blocks.byCard[key]) blocks.byCard[key] = [];
    const list = blocks.byCard[key];
    list.splice(position ?? list.length, 0, b);
    renumber(list);
    structureChanged();
    return list.find((x) => x.id === b.id) ?? null;
  } catch (e) {
    fail(e);
    return null;
  }
}

/** À appeler après avoir modifié `block.data` : enregistre après un court délai. */
export function saveBlock(block: Block) {
  const id = block.id;
  const push = () => invoke<void>("block_update", { id, data: $state.snapshot(block.data) }).catch(fail);
  // Blocs affichés aussi dans la fenêtre compacte (hors vidéo, dont les données ne changent pas en jeu)
  if (block.kind === "counter" || block.kind === "timer" || block.kind === "list")
    send("block-changed", { block: $state.snapshot(block) as Block });
  clearTimeout(pending.get(id)?.timer);
  pending.set(id, {
    send: push,
    timer: setTimeout(() => {
      pending.delete(id);
      push();
    }, SAVE_DELAY_MS),
  });
}

/** Envoie tout de suite les modifications en attente (changement de jeu, fermeture). */
export async function flushBlocks() {
  const all = [...pending.values()];
  pending.clear();
  for (const p of all) clearTimeout(p.timer);
  await Promise.all(all.map((p) => p.send()));
}

export async function deleteBlock(block: Block) {
  const p = pending.get(block.id);
  if (p) {
    clearTimeout(p.timer);
    pending.delete(block.id);
  }
  const snapshot = $state.snapshot(block) as Block;
  const key = cardKey(block);
  const list = blocks.byCard[key] ?? [];
  const index = list.findIndex((b) => b.id === block.id);
  try {
    await invoke("block_delete", { id: block.id });
  } catch (e) {
    fail(e);
    return;
  }
  if (index >= 0) list.splice(index, 1);
  renumber(list);
  structureChanged();
  showToast(`Bloc « ${kindInfo(snapshot.kind).label} » supprimé`, () =>
    createBlock(key, snapshot.kind, snapshot.data, index),
  );
}

export function duplicateBlock(block: Block) {
  const key = cardKey(block);
  const list = blocks.byCard[key] ?? [];
  const index = list.findIndex((b) => b.id === block.id);
  const data = $state.snapshot(block.data);
  // Un chronomètre dupliqué repart à l'arrêt
  if (block.kind === "timer") Object.assign(data, { running_since: null });
  return createBlock(key, block.kind, data, index + 1);
}

/** Nouvel ordre après un glisser-déposer. */
export function reorderBlocks(key: CardKey, ordered: Block[]) {
  blocks.byCard[key] = ordered;
  renumber(ordered);
  const o = ownerOf(key);
  invoke("blocks_reorder", { achievementId: o.achievement_id, cardId: o.card_id, ids: ordered.map((b) => b.id) }).then(
    structureChanged,
    fail,
  );
}

/** Carte libre supprimée : ses blocs disparaissent aussi de l'affichage. */
export function forgetCard(key: CardKey) {
  for (const b of blocks.byCard[key] ?? []) {
    clearTimeout(pending.get(b.id)?.timer);
    pending.delete(b.id);
  }
  delete blocks.byCard[key];
}

function renumber(list: Block[]) {
  list.forEach((b, i) => (b.position = i));
}

/** Durée d'affichage d'une notification à voir, une fois la fenêtre revenue au premier plan. */
const SEEN_MS = 10000;

let toastTimer: ReturnType<typeof setTimeout> | undefined;
/** Attente du retour de la fenêtre au premier plan (notification `untilSeen`). */
let stopWaiting: (() => void) | null = null;

/**
 * Affiche une notification. `untilSeen` : si la fenêtre n'est pas au premier plan (le joueur est en jeu),
 * elle reste affichée jusqu'à son retour, puis encore quelques secondes (succès débloqué en jeu).
 */
export function showToast(message: string, undo: (() => void) | null = null, { untilSeen = false } = {}) {
  toast.message = message;
  toast.undo = undo;
  clearTimeout(toastTimer);
  stopWaiting?.();
  stopWaiting = null;
  const hide = (ms: number) => {
    toastTimer = setTimeout(() => {
      toast.message = "";
      toast.undo = null;
    }, ms);
  };
  if (untilSeen && !document.hasFocus()) {
    const onFocus = () => {
      stopWaiting?.();
      stopWaiting = null;
      hide(SEEN_MS);
    };
    window.addEventListener("focus", onFocus);
    stopWaiting = () => window.removeEventListener("focus", onFocus);
  } else {
    hide(untilSeen ? SEEN_MS : UNDO_MS);
  }
}

export function dismissToast() {
  clearTimeout(toastTimer);
  stopWaiting?.();
  stopWaiting = null;
  toast.message = "";
  toast.undo = null;
}

/** Enregistre une image (fichier, collage, glisser) et renvoie son chemin relatif. */
export async function uploadImage(file: Blob): Promise<string> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  return invoke<string>("image_save", bytes);
}

/** Première image trouvée dans un presse-papiers ou un glisser. */
export function imageFrom(dt: DataTransfer | null): File | null {
  if (!dt) return null;
  for (const item of dt.items ?? []) {
    if (item.kind === "file" && item.type.startsWith("image/")) return item.getAsFile();
  }
  for (const f of dt.files ?? []) if (f.type.startsWith("image/")) return f;
  return null;
}
