<script lang="ts">
  import { onMount } from "svelte";
  import AboutDialog from "./lib/AboutDialog.svelte";
  import AddGameDialog from "./lib/AddGameDialog.svelte";
  import Celebration from "./lib/Celebration.svelte";
  import GameView from "./lib/GameView.svelte";
  import Home from "./lib/Home.svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import { api, completedSlot, importSummary, isComplete, type GameSummary } from "./lib/library";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { flushBlocks, listenBlockChanges, showToast } from "./lib/blocks/store.svelte";
  import SettingsDialog from "./lib/SettingsDialog.svelte";
  import { loadSettings } from "./lib/settings.svelte";
  import { applyMotionAttribute, loadMotion } from "./lib/motion.svelte";
  import { applyTheme, loadAppearance } from "./lib/theme.svelte";
  import { chime, loadSound } from "./lib/sound.svelte";
  import { loadShortcuts, shortcuts, type ShortcutAction } from "./lib/shortcuts.svelte";
  import { on, send } from "./lib/sync";
  import { loadTags } from "./lib/tags.svelte";
  import Toast from "./lib/Toast.svelte";
  import UpdateBanner from "./lib/UpdateBanner.svelte";
  import { checkOnStartup, loadUpdatePrefs } from "./lib/update.svelte";

  let games = $state<GameSummary[]>([]);
  let imagesDir = $state("");
  let selected = $state<number | null>(null);
  let adding = $state(false);
  let settingsOpen = $state(false);
  let aboutOpen = $state(false);
  let loadError = $state("");
  /** Page affichée à droite du panneau : le jeu choisi ou l'accueil. */
  let page = $state<"game" | "home">("game");
  /** Jeu dont la célébration du 100 % est affichée. */
  let celebrating = $state<number | null>(null);

  const current = $derived(games.find((g) => g.appid === selected) ?? null);

  onMount(async () => {
    // Avant la fermeture : envoyer les modifications de blocs encore en attente
    try {
      getCurrentWindow().onCloseRequested(() => flushBlocks());
    } catch {
      /* hors de Tauri (interface simulée) */
    }
    try {
      let last: string | null;
      [imagesDir, games, last] = await Promise.all([
        api.imagesDir(),
        api.list(),
        api.getSetting("last_game").catch(() => null),
        loadSettings(),
        loadTags(),
        loadMotion(),
        loadSound(),
        loadAppearance(),
        loadUpdatePrefs(),
      ]);
      // Dernier jeu consulté, sinon le premier du panneau. Bibliothèque vide (premier lancement) :
      // page d'accueil de bienvenue.
      selected = games.find((g) => g.appid === Number(last))?.appid ?? games[0]?.appid ?? null;
    } catch (e) {
      loadError = String(e);
    }
    // Raccourcis globaux : réglés ici, enregistrés par la fenêtre compacte quand elle est ouverte
    loadShortcuts();
    // Nouvelle version publiée ? (quelques secondes après le démarrage, si le réglage le permet)
    checkOnStartup();
  });

  // Fenêtre compacte : compteurs, chronos et listes modifiés, coches, demande du jeu ouvert, raccourcis
  $effect(() => {
    const stops = [
      listenBlockChanges(),
      on("achievement-changed", ({ game }) => upsert(game)),
      on("compact-ready", () => selected != null && send("compact-game", { appid: selected })),
      // Succès débloqué en jeu (actualisation automatique) : liste, cartes, 100 % mis à jour
      on("steam-progress", ({ game, unlocked }) => {
        upsert(game);
        if (unlocked.length === 0) return;
        const names = unlocked.map((u) => `« ${u.name} »`).join(", ");
        // Reste affichée jusqu'au retour dans la fenêtre (Alt+Tab depuis le jeu), puis 10 s
        showToast(`🏆 ${unlocked.length > 1 ? "Succès débloqués" : "Succès débloqué"} dans ${game.name} : ${names}`, null, {
          untilSeen: true,
        });
        // Fenêtre compacte ouverte : c'est elle qui sonne (un seul carillon)
        if (!shortcuts.live) chime();
      }),
      on("shortcuts-status", ({ errors, live }) => {
        shortcuts.errors = errors as Partial<Record<ShortcutAction, string>>;
        shortcuts.live = live;
      }),
    ];
    return () => stops.forEach((s) => s());
  });
  $effect(() => {
    if (selected != null) send("compact-game", { appid: selected });
  });
  // Rouvert au prochain démarrage
  $effect(() => {
    if (selected != null) api.setSetting("last_game", String(selected)).catch(() => {});
  });
  $effect(applyMotionAttribute);
  $effect(applyTheme);

  function upsert(g: GameSummary) {
    const i = games.findIndex((x) => x.appid === g.appid);
    const newlyComplete = (i < 0 || !isComplete(games[i])) && isComplete(g);
    // Le jeu vient de passer à 100 % (coche, fenêtre compacte, actualisation) ou vient d'être ajouté
    // déjà terminé : célébration, une fois par jeu.
    if (newlyComplete && !g.celebrated) {
      g = { ...g, celebrated: true };
      celebrating = g.appid;
      api.markCelebrated(g.appid).catch(() => {});
    }
    // Ordre de la liste, comme le cœur Rust : nouveau jeu en haut ; jeu qui atteint 100 % (ou ajouté
    // déjà terminé) juste après le dernier jeu en cours
    if (newlyComplete) {
      const rest = games.filter((x) => x.appid !== g.appid);
      rest.splice(completedSlot(rest), 0, g);
      games = rest;
    } else if (i >= 0) games[i] = g;
    else games.unshift(g);
  }

  /** Ordre choisi dans le panneau de gauche (liste complète, jeux masqués compris). */
  function reorder(order: GameSummary[]) {
    games = order;
    api.reorder(order.map((g) => g.appid)).catch((e) => showToast(`Ordre des jeux non enregistré : ${e}`));
  }

  function onImported(g: GameSummary) {
    upsert(g);
    select(g.appid);
    adding = false;
  }

  /** Version de la vue du jeu : changée après un import, pour tout recharger (cartes, blocs, étiquettes). */
  let viewVersion = $state(0);

  /** Importe un guide choisi par le joueur, puis ouvre son jeu (ajouté depuis Steam s'il manquait). */
  async function importGuide() {
    adding = false;
    try {
      await flushBlocks();
      const report = await api.guideImport();
      if (!report) return;
      upsert(report.game);
      select(report.game.appid);
      viewVersion++;
      send("data-changed", { appid: report.game.appid });
      showToast(importSummary(report));
    } catch (e) {
      showToast(`Import impossible : ${e}`);
    }
  }

  function select(appid: number) {
    selected = appid;
    page = "game";
  }

  function onDeleted(appid: number) {
    games = games.filter((g) => g.appid !== appid);
    if (selected === appid) selected = games[0]?.appid ?? null;
  }
</script>

<div class="flex h-screen">
  <Sidebar
    {games}
    {imagesDir}
    {selected}
    {page}
    onselect={select}
    onreorder={reorder}
    onhome={() => (page = "home")}
    onadd={() => (adding = true)}
    onsettings={() => (settingsOpen = true)}
    onabout={() => (aboutOpen = true)}
  />

  {#if current && page === "game"}
    {#key viewVersion}
      <GameView
        game={current}
        {imagesDir}
        onchanged={upsert}
        ondeleted={onDeleted}
        oncelebrate={() => (celebrating = selected)}
        onimport={importGuide}
      />
    {/key}
  {:else}
    <Home {games} {imagesDir} {loadError} onopen={select} onadd={() => (adding = true)} onimport={importGuide} />
  {/if}
</div>

<Toast />
<UpdateBanner />

{#if settingsOpen}
  <SettingsDialog onclose={() => (settingsOpen = false)} />
{/if}

{#if aboutOpen}
  <AboutDialog onclose={() => (aboutOpen = false)} />
{/if}

{#if adding}
  <AddGameDialog onclose={() => (adding = false)} onimported={onImported} onimportguide={importGuide} />
{/if}

{#if celebrating != null}
  {#key celebrating}
    <Celebration
      appid={celebrating}
      {imagesDir}
      onclose={() => (celebrating = null)}
      onremoved={(appid) => {
        celebrating = null;
        onDeleted(appid);
      }}
    />
  {/key}
{/if}
