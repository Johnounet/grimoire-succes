<script lang="ts">
  import { onMount } from "svelte";
  import AddGameDialog from "./lib/AddGameDialog.svelte";
  import GameView from "./lib/GameView.svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import { api, type GameSummary } from "./lib/library";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { flushBlocks, listenBlockChanges } from "./lib/blocks/store.svelte";
  import SettingsDialog from "./lib/SettingsDialog.svelte";
  import { loadSettings } from "./lib/settings.svelte";
  import { active, applyShortcuts, loadActive, loadShortcuts } from "./lib/shortcuts.svelte";
  import { on, send } from "./lib/sync";
  import { loadTags } from "./lib/tags.svelte";
  import Toast from "./lib/Toast.svelte";

  let games = $state<GameSummary[]>([]);
  let imagesDir = $state("");
  let selected = $state<number | null>(null);
  let adding = $state(false);
  let settingsOpen = $state(false);
  let loadError = $state("");

  const current = $derived(games.find((g) => g.appid === selected) ?? null);

  onMount(async () => {
    // Avant la fermeture : envoyer les modifications de blocs encore en attente
    try {
      getCurrentWindow().onCloseRequested(() => flushBlocks());
    } catch {
      /* hors de Tauri (interface simulée) */
    }
    try {
      [imagesDir, games] = await Promise.all([api.imagesDir(), api.list(), loadSettings(), loadTags(), loadActive()]);
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

  function upsert(g: GameSummary) {
    const i = games.findIndex((x) => x.appid === g.appid);
    if (i >= 0) games[i] = g;
    else games.push(g);
  }

  function onImported(g: GameSummary) {
    upsert(g);
    selected = g.appid;
    adding = false;
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
    onselect={(id) => (selected = id)}
    onadd={() => (adding = true)}
    onsettings={() => (settingsOpen = true)}
  />

  {#if current}
    <GameView game={current} {imagesDir} onchanged={upsert} ondeleted={onDeleted} />
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

{#if adding}
  <AddGameDialog onclose={() => (adding = false)} onimported={onImported} />
{/if}
