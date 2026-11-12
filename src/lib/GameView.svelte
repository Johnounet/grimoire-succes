<script lang="ts">
  import { api, heroUrl, percent, sourceLabel, type AchievementRow, type GameSummary } from "./library";
  import { SvelteSet } from "svelte/reactivity";
  import AchievementList from "./AchievementList.svelte";
  import { blocks, loadBlocks } from "./blocks/store.svelte";
  import type { Column, WorkspaceItem } from "./library";
  import ProgressRing from "./ProgressRing.svelte";
  import { setPanelOpen, settings } from "./settings.svelte";
  import Workspace from "./Workspace.svelte";

  let {
    game,
    imagesDir,
    onchanged,
    ondeleted,
  }: {
    game: GameSummary;
    imagesDir: string;
    onchanged: (g: GameSummary) => void;
    ondeleted: (appid: number) => void;
  } = $props();

  let achievements = $state<AchievementRow[]>([]);
  let refreshing = $state(false);
  let confirmDelete = $state(false);
  let error = $state("");
  let heroOk = $state(true);
  let workspace = $state.raw<WorkspaceItem[]>([]);
  /** Succès cachés révélés (liste et cartes), oubliés en changeant de jeu. */
  const revealed = new SvelteSet<number>();

  const byId = $derived(new Map(achievements.map((a) => [a.id, a])));
  const personalOrder = $derived(new Map(workspace.map((it, i) => [it.achievement_id, i])));

  // Valeurs dérivées : elles ne changent que si la valeur change, pas à chaque mise à jour de `game`
  // (une coche renvoie un nouveau `game`, qui ne doit pas recharger la liste).
  const appidKey = $derived(game.appid);
  const refreshedKey = $derived(game.refreshed_at);

  $effect(() => {
    const appid = appidKey;
    void refreshedKey; // recharge aussi après une actualisation
    heroOk = true;
    confirmDelete = false;
    error = "";
    api.achievements(appid).then((a) => {
      if (appid === game.appid) achievements = a;
    });
  });

  // L'espace de travail ne dépend que du jeu (une actualisation n'y touche pas)
  $effect(() => {
    const appid = appidKey;
    revealed.clear();
    workspace = [];
    loadBlocks(appid);
    api.workspaceList(appid).then((w) => {
      if (appid === game.appid) workspace = w;
    });
  });

  // Enregistrements de l'espace mis en file, pour qu'ils arrivent dans l'ordre
  let saving: Promise<unknown> = Promise.resolve();
  function saveWorkspace(items: WorkspaceItem[]) {
    const appid = game.appid;
    workspace = items;
    saving = saving
      .then(() => api.workspaceSave(appid, items))
      .catch((e) => (error = `Enregistrement de l'espace de travail impossible : ${e}`));
  }

  function addToWorkspace(ids: number[], column: Column = "todo") {
    const present = new Set(workspace.map((it) => it.achievement_id));
    const fresh = ids.filter((id) => byId.has(id) && !present.has(id));
    if (fresh.length === 0) return;
    saveWorkspace([...workspace, ...fresh.map((id) => ({ achievement_id: id, column, collapsed: false }))]);
  }

  const pct = $derived(percent(game));

  async function refresh() {
    refreshing = true;
    error = "";
    try {
      onchanged(await api.importGame(game.appid));
    } catch (e) {
      error = `Actualisation impossible : ${e}`;
    } finally {
      refreshing = false;
    }
  }

  async function remove() {
    try {
      await api.deleteGame(game.appid);
      ondeleted(game.appid);
    } catch (e) {
      error = `Suppression impossible : ${e}`;
    }
  }

  /** Coche instantanée à l'écran, enregistrée ensuite ; annulée si l'enregistrement échoue. */
  async function toggle(a: AchievementRow) {
    const row = achievements.find((x) => x.id === a.id);
    if (!row) return;
    const before = { unlocked: row.unlocked, manual: row.manual, unlock_time: row.unlock_time };
    row.unlocked = !row.unlocked;
    row.manual = true;
    if (!row.unlocked) row.unlock_time = null;
    try {
      onchanged(await api.setUnlocked(row.id, row.unlocked));
    } catch (e) {
      Object.assign(row, before);
      error = `Enregistrement impossible : ${e}`;
    }
  }
</script>

<section class="flex h-screen min-w-0 flex-1 flex-col overflow-hidden">
  <!-- Bannière -->
  <header class="relative h-40 shrink-0 overflow-hidden bg-raised">
    {#if heroOk}
      <img
        src={heroUrl(game.appid)}
        alt=""
        class="absolute inset-0 h-full w-full object-cover"
        onerror={() => (heroOk = false)}
      />
    {/if}
    <div class="absolute inset-0 bg-gradient-to-t from-bg via-bg/50 to-transparent"></div>
    <div class="absolute inset-x-0 bottom-0 flex items-end gap-4 px-6 pb-4">
      <div class="min-w-0 flex-1">
        <h1 class="truncate text-3xl font-bold drop-shadow">{game.name}</h1>
        <p class="mt-1 text-sm text-fg/80">
          <span class="font-semibold {pct >= 100 ? 'text-gold' : ''}">{game.unlocked} / {game.total}</span>
          succès · {pct} %
          <span class="text-muted">· importé depuis le {sourceLabel[game.source]}</span>
          {#if !game.has_api_names}<span class="text-muted">· certains ID techniques manquent</span>{/if}
        </p>
      </div>
      <ProgressRing value={pct} size={48} />
      <button
        onclick={refresh}
        disabled={refreshing}
        class="rounded-lg border border-line bg-panel/80 px-3 py-1.5 text-sm backdrop-blur transition hover:bg-raised disabled:opacity-60"
      >
        {refreshing ? "Actualisation…" : "Actualiser"}
      </button>
      <button
        onclick={() => (confirmDelete = true)}
        class="rounded-lg border border-line bg-panel/80 px-3 py-1.5 text-sm text-muted backdrop-blur transition hover:text-danger"
        aria-label="Supprimer le jeu"
        title="Supprimer le jeu"
      >
        Supprimer
      </button>
    </div>
  </header>

  {#if confirmDelete || error || blocks.error}
    <div class="space-y-2 px-6 pt-3">
      {#if confirmDelete}
        <div class="flex flex-wrap items-center gap-3 rounded-lg border border-danger/40 bg-danger/10 p-3 text-sm">
          <span class="flex-1">
            Supprimer « {game.name} » de la bibliothèque ? Ses succès, son espace de travail et ses guides seront effacés.
            <span class="text-muted">(L'export des guides avant suppression arrivera avec les guides.)</span>
          </span>
          <button onclick={() => (confirmDelete = false)} class="rounded-md px-3 py-1 hover:bg-raised">Annuler</button>
          <button onclick={remove} class="rounded-md bg-danger px-3 py-1 font-medium text-white">Supprimer</button>
        </div>
      {/if}
      {#if error}
        <p class="rounded-lg bg-danger/10 p-3 text-sm text-danger">{error}</p>
      {/if}
      {#if blocks.error}
        <p class="flex items-center gap-3 rounded-lg bg-danger/10 p-3 text-sm text-danger">
          <span class="flex-1">{blocks.error}</span>
          <button onclick={() => (blocks.error = "")} class="text-muted hover:text-fg" aria-label="Fermer">✕</button>
        </p>
      {/if}
    </div>
  {/if}

  <div class="flex min-h-0 flex-1">
    <!-- Centre : espace de travail -->
    <div class="min-w-0 flex-1 overflow-y-auto">
      <Workspace
        items={workspace}
        {byId}
        {revealed}
        {imagesDir}
        ontoggle={toggle}
        onchange={saveWorkspace}
        onadd={addToWorkspace}
      />
    </div>

    <!-- Droite : liste complète, repliable -->
    {#if settings.panelOpen}
      <aside class="flex w-[clamp(320px,34vw,480px)] shrink-0 flex-col border-l border-line bg-panel" aria-label="Tous les succès">
        <div class="flex shrink-0 items-center justify-between border-b border-line px-3 py-2">
          <h2 class="text-sm font-semibold">Tous les succès</h2>
          <button
            onclick={() => setPanelOpen(false)}
            class="rounded px-2 py-0.5 text-muted hover:bg-raised hover:text-fg"
            aria-label="Replier la liste des succès"
            title="Replier"
          >
            ›
          </button>
        </div>
        <div class="min-h-0 flex-1 overflow-y-auto">
          {#key game.appid}
            <AchievementList {achievements} {personalOrder} {revealed} ontoggle={toggle} onadd={(ids) => addToWorkspace(ids)} />
          {/key}
        </div>
      </aside>
    {:else}
      <button
        onclick={() => setPanelOpen(true)}
        class="flex w-10 shrink-0 flex-col items-center gap-3 border-l border-line bg-panel py-4 text-muted transition hover:bg-raised hover:text-fg"
        aria-label="Afficher la liste des succès"
        title="Afficher la liste des succès"
      >
        <span>‹</span>
        <span class="text-xs tracking-wide [writing-mode:vertical-rl]">Tous les succès</span>
      </button>
    {/if}
  </div>
</section>
