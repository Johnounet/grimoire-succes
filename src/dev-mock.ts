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
  // Progression suivie par Steam (cache local) : sur « Le pont le vit » seulement
  progress: api_name === "ACH.BRIDGE" ? { value: 37, min: 0, max: 50 } : null,
}));

const cardSizes = new Map<number, { w: number; h: number }>();
type MockItem = { achievement_id: number | null; card_id: number | null; grid: { w: number; h: number } | null };
let workspace: MockItem[] = [];
let freeCards: { id: number; title: string }[] = [];
let nextFreeId = 1;
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
type MockBlock = { id: number; achievement_id: number | null; card_id: number | null; kind: string; position: number; data: unknown };
let blocks: MockBlock[] = [];
let nextBlockId = 1;
/** Comme `cardKey` de l'interface : succès positif, carte libre négative. */
const ownerKey = (o: { achievement_id: number | null; card_id: number | null }) => (o.card_id != null ? -o.card_id : (o.achievement_id ?? 0));
const argsKey = (args: Record<string, unknown>) =>
  ownerKey({ achievement_id: (args.achievementId as number | null) ?? null, card_id: (args.cardId as number | null) ?? null });

function renumber(key: number) {
  blocks
    .filter((b) => ownerKey(b) === key)
    .sort((a, b) => a.position - b.position)
    .forEach((b, i) => (b.position = i));
}

let completedAt: number | null = null;
let celebrated = false;
/** Comme `update_completion` côté Rust. */
function updateCompletion(when: number | null) {
  const done = achievements.every((a) => a.unlocked);
  if (!done) completedAt = null;
  else completedAt ??= when ?? Math.floor(Date.now() / 1000);
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
  completed_at: completedAt,
  playtime_minutes: 1504,
  celebrated,
});

/** Autres jeux du panneau (affichage et ordre seulement ; ouverts, ils montrent les succès de Portal 2). */
const otherGames = [
  { appid: 1145360, name: "Hades", total: 49, unlocked: 31, completed_at: null },
  { appid: 2379780, name: "Balatro", total: 30, unlocked: 30, completed_at: 1_760_000_000 },
  { appid: 1794680, name: "Vampire Survivors", total: 220, unlocked: 87, completed_at: null },
].map((g) => ({ ...g, cover_file: null, cover_kind: null, source: "local_cache", has_api_names: true, refreshed_at: 1, playtime_minutes: null, celebrated: true }));
let gameOrder = [620, ...otherGames.map((g) => g.appid)];
const allGames = () => gameOrder.map((id) => (id === 620 ? summary() : otherGames.find((g) => g.appid === id)!));

/** Sauvegardes simulées : quelques automatiques des derniers jours. */
let backups = [0, 1, 2].map((d) => ({
  id: `${d}-auto`,
  created_at: Math.floor(Date.now() / 1000) - d * 86400 - 3600,
  kind: "auto",
  games: 4,
  size: 2_900_000 + d * 1000,
}));

/** Fenêtre compacte simulée (onglet « ?compact », données propres à l'onglet) : une carte épinglée
 *  de démonstration avec un élément de chaque sorte, et « Le pont le vit » épinglé avec un compteur (il a une progression Steam). */
function seedCompact() {
  freeCards = [{ id: nextFreeId++, title: "Route générale" }];
  const free = freeCards[0].id;
  const add = (owner: { achievement_id: number | null; card_id: number | null }, kind: string, data: unknown) =>
    blocks.push({ id: nextBlockId++, ...owner, kind, position: blocks.filter((b) => ownerKey(b) === ownerKey(owner)).length, data });
  const card = { achievement_id: null, card_id: free };
  add(card, "counter", { label: "Cubes de stockage", value: 3, goal: 12, step: 1 });
  add(card, "timer", { label: "Chambre 9", target_ms: 70000, best_ms: 82000, elapsed_ms: 0, running_since: null });
  add(card, "list", { items: ["Radio 1", "Radio 2", "Radio 3", "Radio 4"].map((text, i) => ({ text, checked: i === 0 })) });
  add(card, "video", { url: "https://www.youtube.com/watch?v=jNQXAC9IVRw", markers: [{ time: 5, label: "Les éléphants" }] });
  const ach = { achievement_id: 4, card_id: null };
  add(ach, "counter", { label: "", value: 0, goal: null, step: 1 });
  workspace = [
    { ...card, grid: null, column: "todo", collapsed: false, pinned: true } as MockItem,
    { ...ach, grid: null, column: "todo", collapsed: false, pinned: true } as MockItem,
  ];
}

export function installMock(label = "main") {
  mockConvertFileSrc("windows");
  mockWindows(label);
  if (label === "compact") seedCompact();
  mockIPC(
    (cmd, payload) => {
    const args = (payload ?? {}) as Record<string, unknown>;
    switch (cmd) {
      case "images_dir":
        return "C:\\mock\\images";
      case "library_list":
        // http://localhost:1420/?empty : bibliothèque vide (page de bienvenue du premier lancement)
        return new URLSearchParams(location.search).has("empty") ? [] : allGames();
      case "backups_list":
        return [...backups].sort((x, y) => y.created_at - x.created_at);
      case "backup_create": {
        const b = { id: `${Date.now()}-manuel`, created_at: Math.floor(Date.now() / 1000), kind: "manuel", games: 4, size: 3_400_000 };
        backups.push(b);
        return b;
      }
      case "backup_delete":
        backups = backups.filter((b) => b.id !== args.id);
        return null;
      case "backup_restore":
      case "backups_open_folder":
        return null;
      case "library_delete":
        gameOrder = gameOrder.filter((id) => id !== args.appid);
        return null;
      case "library_reorder":
        gameOrder = [...(args.appids as number[])];
        return null;
      case "library_local_suggestions":
      case "library_search":
        return [];
      case "game_achievements":
        return achievements.map((a) => ({ ...a }));
      case "achievements_lock_all":
        achievements.forEach((a) => Object.assign(a, { unlocked: false, manual: true, unlock_time: null }));
        updateCompletion(null);
        return summary();
      case "achievement_set_unlocked": {
        const a = achievements.find((x) => x.id === args.id);
        if (a)
          Object.assign(a, {
            unlocked: args.unlocked,
            manual: true,
            unlock_time: args.unlocked ? (a.unlock_time ?? Math.floor(Date.now() / 1000)) : null,
          });
        updateCompletion(Math.floor(Date.now() / 1000));
        return summary();
      }
      case "game_recap": {
        const rarest = achievements
          .filter((a) => a.unlocked && a.rarity != null)
          .sort((a, b) => (a.rarity as number) - (b.rarity as number))[0];
        return { game: summary(), rarest: rarest ? { ...rarest } : null };
      }
      case "home_recent_unlocks":
        return achievements
          .filter((a) => a.unlocked && a.unlock_time != null)
          .map((a) => ({ id: a.id, appid: 620, game_name: summary().name, name: a.name, icon_url: a.icon_url, rarity: a.rarity, unlock_time: a.unlock_time }))
          .sort((x, y) => y.unlock_time! - x.unlock_time!)
          .slice(0, args.limit as number);
      case "game_mark_celebrated":
        celebrated = true;
        return null;
      case "workspace_list":
        return workspace;
      case "guide_export":
        console.info("[mock] guide_export", args.appid, args.keepState);
        return String.raw`C:\Users\vous\Documents\Portal 2 - guide.json`;
      case "guide_import":
        return {
          game: summary(),
          blocks: 4,
          free_cards: 1,
          placed: 2,
          duplicates: 1,
          unmatched: ["Succès d'un DLC"],
        };
      case "free_cards_list":
        return freeCards.map((f) => ({ ...f }));
      case "free_card_create": {
        const f = { id: nextFreeId++, title: String(args.title).trim() };
        freeCards.push(f);
        return { ...f };
      }
      case "free_card_rename": {
        const f = freeCards.find((x) => x.id === args.id);
        if (f) f.title = String(args.title).trim();
        return null;
      }
      case "free_card_delete":
        freeCards = freeCards.filter((f) => f.id !== args.id);
        workspace = workspace.filter((it) => it.card_id !== args.id);
        blocks = blocks.filter((b) => b.card_id !== args.id);
        return null;
      case "workspace_card_sizes":
        return [...cardSizes].map(([achievement_id, size]) => ({ achievement_id, ...size }));
      case "workspace_save":
        workspace = (args.items as MockItem[]).filter((it) => it.card_id == null || freeCards.some((f) => f.id === it.card_id));
        for (const it of workspace) if (it.grid && it.achievement_id != null) cardSizes.set(it.achievement_id, { w: it.grid.w, h: it.grid.h });
        console.info("[mock] workspace_save", JSON.stringify(workspace));
        return null;
      case "block_get": {
        const b = blocks.find((x) => x.id === args.id);
        const name = b && (achievements.find((x) => x.id === b.achievement_id)?.name ?? freeCards.find((f) => f.id === b.card_id)?.title);
        return b && name != null ? { block: structuredClone(b), appid: 620, card_title: name } : null;
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
        return [...blocks].sort((a, b) => ownerKey(a) - ownerKey(b) || a.position - b.position);
      case "block_create": {
        const key = argsKey(args);
        const mine = blocks.filter((b) => ownerKey(b) === key);
        const pos = (args.position as number | null) ?? mine.length;
        mine.filter((b) => b.position >= pos).forEach((b) => b.position++);
        const b = {
          id: nextBlockId++,
          achievement_id: (args.achievementId as number | null) ?? null,
          card_id: (args.cardId as number | null) ?? null,
          kind: args.kind as string,
          position: pos,
          data: args.data,
        };
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
        if (b) renumber(ownerKey(b));
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
      case "app_mode":
        return { portable: new URLSearchParams(location.search).has("portable") };
      // ?update : une nouvelle version est annoncée
      case "update_check":
        return new URLSearchParams(location.search).has("update")
          ? { version: "0.2.0", notes: "Corrections et nouveautés.", canInstall: !new URLSearchParams(location.search).has("portable") }
          : null;
      case "update_install":
        throw new Error("simulation : rien à installer");
      case "update_open_page":
        console.info("[mock] update_open_page");
        return null;
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
