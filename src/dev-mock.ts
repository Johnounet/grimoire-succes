// Simulateur du cœur Rust, pour ouvrir l'interface dans un simple navigateur (http://localhost:1420)
// pendant le développement. Chargé seulement en mode dev ET hors de Tauri : absent des versions publiées.
import { mockConvertFileSrc, mockIPC, mockWindows } from "@tauri-apps/api/mocks";

const icon = (f: string) => `https://cdn.fastly.steamstatic.com/steamcommunity/public/images/apps/620/${f}`;

const achievements = [
  ["ACH.SURVIVE_CONTAINER_RIDE", "Debout là-dedans", "Survivre à la commande manuelle", 74, true, false],
  ["ACH.WAKE_UP", "Monstre", "Retrouver GLaDOS", 64.7, true, false],
  ["ACH.LASER", "Pas découragée", "Terminer le premier test du rayon thermique", 62.6, false, false],
  ["ACH.BRIDGE", "Le pont le vit", "Terminer le test du pont de lumière", 53.2, false, false],
  ["ACH.SECRET", "Le secret de Wheatley", "Découvrir ce que cache Wheatley", 3.7, false, true],
  ["ACH.SPEED", "Rapide comme l'éclair", "Finir la chambre 9 en moins de 70 secondes", 12.4, false, false],
].map(([api_name, name, description, rarity, unlocked, hidden], i) => ({
  id: i + 1,
  api_name,
  name,
  description,
  hidden,
  icon_url: icon(`${String(api_name).slice(4)}.jpg`),
  icon_gray_url: icon(`${String(api_name).slice(4)}_BW.jpg`),
  rarity,
  unlocked,
  unlock_time: unlocked ? 1369056684 + i * 3600 : null,
  manual: false,
  dlc_appid: null,
  removed: false,
}));

const cardSizes = new Map<number, { w: number; h: number }>();
let workspace: { achievement_id: number; column: string; collapsed: boolean; pinned?: boolean }[] = [];
let tags = [
  ["ratable", "#ef5b5b"],
  ["grind", "#f2994a"],
  ["en ligne", "#4f7cff"],
  ["difficile", "#b07cff"],
  ["cumulable", "#4cc38a"],
  ["DLC", "#f2c14e"],
].map(([name, color], i) => ({ id: i + 1, name, color }));
let nextTagId = tags.length + 1;
let achievementTags: { achievement_id: number; tag_id: number }[] = [];
const settings = new Map<string, string>();
type MockBlock = { id: number; achievement_id: number; kind: string; position: number; data: unknown };
let blocks: MockBlock[] = [];
let nextBlockId = 1;

function renumber(achievementId: number) {
  blocks
    .filter((b) => b.achievement_id === achievementId)
    .sort((a, b) => a.position - b.position)
    .forEach((b, i) => (b.position = i));
}

const summary = () => ({
  appid: 620,
  name: "Portal 2 (données simulées)",
  cover_file: null,
  cover_kind: null,
  source: "local_cache",
  total: achievements.length,
  unlocked: achievements.filter((a) => a.unlocked).length,
  has_api_names: true,
  refreshed_at: 1,
});

export function installMock(label = "main") {
  mockConvertFileSrc("windows");
  mockWindows(label);
  mockIPC(
    (cmd, payload) => {
    const args = (payload ?? {}) as Record<string, unknown>;
    switch (cmd) {
      case "images_dir":
        return "C:\\mock\\images";
      case "library_list":
        return [summary()];
      case "library_local_suggestions":
      case "library_search":
        return [];
      case "game_achievements":
        return achievements.map((a) => ({ ...a }));
      case "achievements_lock_all":
        achievements.forEach((a) => Object.assign(a, { unlocked: false, manual: true, unlock_time: null }));
        return summary();
      case "achievement_set_unlocked": {
        const a = achievements.find((x) => x.id === args.id);
        if (a) Object.assign(a, { unlocked: args.unlocked, manual: true });
        return summary();
      }
      case "workspace_list":
        return workspace;
      case "workspace_card_sizes":
        return [...cardSizes].map(([achievement_id, size]) => ({ achievement_id, ...size }));
      case "workspace_save":
        workspace = args.items as typeof workspace;
        for (const it of args.items as { achievement_id: number; grid: { w: number; h: number } | null }[])
          if (it.grid) cardSizes.set(it.achievement_id, { w: it.grid.w, h: it.grid.h });
        console.info("[mock] workspace_save", JSON.stringify(workspace));
        return null;
      case "block_get": {
        const b = blocks.find((x) => x.id === args.id);
        const a = b && achievements.find((x) => x.id === b.achievement_id);
        return b && a ? { block: structuredClone(b), appid: 620, achievement_name: a.name } : null;
      }
      case "tags_list":
        return tags.map((t) => ({ ...t }));
      case "tag_create": {
        const name = String(args.name).trim();
        if (!name) throw "Le nom de l'étiquette est vide.";
        if (tags.some((t) => t.name.toLowerCase() === name.toLowerCase())) throw "Une étiquette porte déjà ce nom.";
        const t = { id: nextTagId++, name, color: String(args.color) };
        tags.push(t);
        return { ...t };
      }
      case "tag_update": {
        const name = String(args.name).trim();
        if (tags.some((t) => t.id !== args.id && t.name.toLowerCase() === name.toLowerCase())) throw "Une étiquette porte déjà ce nom.";
        const t = tags.find((x) => x.id === args.id);
        if (t) Object.assign(t, { name, color: args.color });
        return null;
      }
      case "tag_delete":
        tags = tags.filter((t) => t.id !== args.id);
        achievementTags = achievementTags.filter((x) => x.tag_id !== args.id);
        return null;
      case "achievement_tags_list":
        return achievementTags;
      case "achievement_tag_set":
        achievementTags = achievementTags.filter((x) => !(x.achievement_id === args.achievementId && x.tag_id === args.tagId));
        if (args.on) achievementTags.push({ achievement_id: args.achievementId as number, tag_id: args.tagId as number });
        return null;
      case "plugin:app|version":
        return "0.1.0-dev";
      case "plugin:global-shortcut|register":
      case "plugin:global-shortcut|unregister":
      case "plugin:global-shortcut|unregister_all":
        return null;
      case "blocks_list":
        return [...blocks].sort((a, b) => a.achievement_id - b.achievement_id || a.position - b.position);
      case "block_create": {
        const achievementId = args.achievementId as number;
        const mine = blocks.filter((b) => b.achievement_id === achievementId);
        const pos = (args.position as number | null) ?? mine.length;
        mine.filter((b) => b.position >= pos).forEach((b) => b.position++);
        const b = { id: nextBlockId++, achievement_id: achievementId, kind: args.kind as string, position: pos, data: args.data };
        blocks.push(b);
        console.info("[mock] block_create", b.kind, b.id);
        return structuredClone(b);
      }
      case "block_update": {
        const b = blocks.find((x) => x.id === args.id);
        if (b) b.data = args.data;
        console.info("[mock] block_update", args.id, JSON.stringify(args.data));
        return null;
      }
      case "block_delete": {
        const b = blocks.find((x) => x.id === args.id);
        blocks = blocks.filter((x) => x.id !== args.id);
        if (b) renumber(b.achievement_id);
        return null;
      }
      case "blocks_reorder":
        (args.ids as number[]).forEach((id, i) => {
          const b = blocks.find((x) => x.id === id);
          if (b) b.position = i;
        });
        return null;
      case "image_save":
        // Le corps brut arrive tel quel : on en fait une URL du navigateur
        return URL.createObjectURL(new Blob([payload as Uint8Array]));
      case "setting_get":
        return settings.get(args.key as string) ?? null;
      case "setting_set":
        settings.set(args.key as string, args.value as string);
        return null;
      default:
        throw new Error(`[mock] commande non simulée : ${cmd}`);
    }
    },
    { shouldMockEvents: true },
  );
}
