<script lang="ts">
  import { onMount } from "svelte";
  import AboutDialog from "./lib/AboutDialog.svelte";
  import AddGameDialog from "./lib/AddGameDialog.svelte";
  import Celebration from "./lib/Celebration.svelte";
  import GameView from "./lib/GameView.svelte";
  import Showcase from "./lib/Showcase.svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import { api, isComplete, type GameSummary } from "./lib/library";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { flushBlocks, listenBlockChanges } from "./lib/blocks/store.svelte";
  import SettingsDialog from "./lib/SettingsDialog.svelte";
  import { loadSettings } from "./lib/settings.svelte";
  import { applyMotionAttribute, loadMotion } from "./lib/motion.svelte";
  import { loadSound } from "./lib/sound.svelte";
  import { active, applyShortcuts, loadActive, loadShortcuts } from "./lib/shortcuts.svelte";
  import { on, send } from "./lib/sync";
  import { loadTags } from "./lib/tags.svelte";
  import Toast from "./lib/Toast.svelte";

  let games = $state<GameSummary[]>([]);
  let imagesDir = $state("");
  let selected = $state<number | null>(null);
  let adding = $state(false);
  let settingsOpen = $state(false);
  let aboutOpen = $state(false);
  let loadError = $state("");
  /** Vitrine des trophées affichée à la place du jeu. */
  let showcase = $state(false);
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
      [imagesDir, games] = await Promise.all([api.imagesDir(), api.list(), loadSettings(), loadTags(), loadActive(), loadMotion(), loadSound()]);
      selected = games[0]?.appid ?? null;
      // Premier lancement : on ouvre directement l'ajout de jeu
      if (games.length === 0) adding = true;
    } catch (e) {
      loadError = String(e);
    }
    // Raccourcis globaux (fenêtre principale seulement)
    loadShortcuts().then(applyShortcuts);
  });

  // Fenêtre compacte : compteurs et chronos modifiés, coches, demande du jeu ouvert
  $effect(() => {
    const stops = [
      listenBlockChanges(),
      on("achievement-changed", ({ game }) => upsert(game)),
      on("compact-ready", () => selected != null && send("compact-game", { appid: selected })),
      // Bloc actif choisi depuis la fenêtre compacte
      on("active-changed", ({ counter, timer }) => {
        active.counter = counter;
        active.timer = timer;
      }),
    ];
    return () => stops.forEach((s) => s());
  });
  $effect(() => {
    if (selected != null) send("compact-game", { appid: selected });
  });
  $effect(applyMotionAttribute);

  function upsert(g: GameSummary) {
    const i = games.findIndex((x) => x.appid === g.appid);
    // Le jeu vient de passer à 100 % (coche, fenêtre compacte, actualisation) : célébration, une fois par jeu.
    // Un jeu importé déjà terminé rejoint la vitrine sans célébration.
    if (i >= 0 && !isComplete(games[i]) && isComplete(g) && !g.celebrated) {
      g = { ...g, celebrated: true };
      celebrating = g.appid;
      api.markCelebrated(g.appid).catch(() => {});
    }
    if (i >= 0) games[i] = g;
    else games.push(g);
  }

  function onImported(g: GameSummary) {
    upsert(g);
    select(g.appid);
    adding = false;
  }

  function select(appid: number) {
    selected = appid;
    showcase = false;
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
    {showcase}
    onselect={select}
    onshowcase={() => (showcase = true)}
    onadd={() => (adding = true)}
    onsettings={() => (settingsOpen = true)}
    onabout={() => (aboutOpen = true)}
  />

  {#if showcase}
    <Showcase {games} {imagesDir} onopen={select} oncelebrate={(id) => (celebrating = id)} />
  {:else if current}
    <GameView
      game={current}
      {imagesDir}
      onchanged={upsert}
      ondeleted={onDeleted}
      oncelebrate={() => (celebrating = selected)}
    />
  {:else}
    <main class="flex flex-1 items-center justify-center p-8 text-center">
      <div class="max-w-sm space-y-3">
        <div class="text-5xl">🏆</div>
        <h1 class="text-xl font-semibold">Chasse aux succès</h1>
        {#if loadError}
          <p class="text-sm text-danger">Impossible d'ouvrir la base de données : {loadError}</p>
        {:else}
          <p class="text-sm text-muted">Ajoutez un jeu pour importer ses succès.</p>
          <button
            onclick={() => (adding = true)}
            class="rounded-lg bg-accent px-4 py-2 text-sm font-medium text-white hover:brightness-110"
          >
            Ajouter un jeu
          </button>
        {/if}
      </div>
    </main>
  {/if}
</div>

<Toast />

{#if settingsOpen}
  <SettingsDialog onclose={() => (settingsOpen = false)} />
{/if}

{#if aboutOpen}
  <AboutDialog onclose={() => (aboutOpen = false)} />
{/if}

{#if adding}
  <AddGameDialog onclose={() => (adding = false)} onimported={onImported} />
{/if}

{#if celebrating != null}
  {#key celebrating}
    <Celebration
      appid={celebrating}
      {imagesDir}
      onclose={() => (celebrating = null)}
      onshowcase={showcase
        ? undefined
        : () => {
            celebrating = null;
            showcase = true;
          }}
    />
  {/key}
{/if}
