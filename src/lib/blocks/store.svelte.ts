// État des blocs du jeu ouvert, et leur enregistrement automatique.
//
// - Une modification s'affiche tout de suite et part en base après un court délai
//   (les frappes rapprochées sont regroupées) ; `flushBlocks()` force l'envoi immédiat.
// - Une suppression peut être annulée quelques secondes (toast « Annuler »).
// - Compteurs et chronos sont aussi affichés dans la fenêtre compacte : chaque changement
//   lui est annoncé (`block-changed`), et ceux qu'elle fait arrivent ici de la même façon.
import { invoke } from "@tauri-apps/api/core";
import { on, send } from "../sync";
import { defaultData, kindInfo, type Block, type BlockKind } from "./types";

const SAVE_DELAY_MS = 400;
const UNDO_MS = 8000;

export const blocks = $state({
  appid: 0,
  /** Blocs par succès, dans l'ordre. */
  byAch: {} as Record<number, Block[]>,
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
  blocks.byAch = {};
  blocks.error = "";
  try {
    const list = await invoke<Block[]>("blocks_list", { appid });
    if (blocks.appid !== appid) return;
    const grouped: Record<number, Block[]> = {};
    for (const b of list) (grouped[b.achievement_id] ??= []).push(b);
    blocks.byAch = grouped;
  } catch (e) {
    blocks.error = `Chargement des blocs impossible : ${e}`;
  }
}

export const blocksOf = (achievementId: number): Block[] => blocks.byAch[achievementId] ?? [];

/** Bloc du jeu ouvert, ou undefined s'il n'est pas chargé ici. */
export function findBlock(id: number): Block | undefined {
  for (const list of Object.values(blocks.byAch)) {
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

export async function createBlock<K extends BlockKind>(
  achievementId: number,
  kind: K,
  data = defaultData(kind),
  position?: number,
): Promise<Block | null> {
  try {
    const b = await invoke<Block>("block_create", { achievementId, kind, data, position: position ?? null });
    // Pas de `??=` ici : sur un état réactif il renverrait le tableau brut, pas sa version suivie
    if (!blocks.byAch[achievementId]) blocks.byAch[achievementId] = [];
    const list = blocks.byAch[achievementId];
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
  if (block.kind === "counter" || block.kind === "timer") send("block-changed", { block: $state.snapshot(block) as Block });
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
  const list = blocks.byAch[block.achievement_id] ?? [];
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
    createBlock(snapshot.achievement_id, snapshot.kind, snapshot.data, index),
  );
}

export function duplicateBlock(block: Block) {
  const list = blocks.byAch[block.achievement_id] ?? [];
  const index = list.findIndex((b) => b.id === block.id);
  const data = $state.snapshot(block.data);
  // Un chronomètre dupliqué repart à l'arrêt
  if (block.kind === "timer") Object.assign(data, { running_since: null });
  return createBlock(block.achievement_id, block.kind, data, index + 1);
}

/** Nouvel ordre après un glisser-déposer. */
export function reorderBlocks(achievementId: number, ordered: Block[]) {
  blocks.byAch[achievementId] = ordered;
  renumber(ordered);
  invoke("blocks_reorder", { achievementId, ids: ordered.map((b) => b.id) }).then(structureChanged, fail);
}

function renumber(list: Block[]) {
  list.forEach((b, i) => (b.position = i));
}

let toastTimer: ReturnType<typeof setTimeout> | undefined;
export function showToast(message: string, undo: (() => void) | null = null) {
  toast.message = message;
  toast.undo = undo;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toast.message = "";
    toast.undo = null;
  }, UNDO_MS);
}

export function dismissToast() {
  clearTimeout(toastTimer);
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
