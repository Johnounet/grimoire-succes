<script lang="ts">
  import {
    api,
    cardSizes,
    heroUrl,
    isComplete,
    percent,
    sourceLabel,
    type AchievementRow,
    type CardKey,
    type FreeCard,
    type GameSummary,
  } from "./library";
  import { untrack } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import AchievementList from "./AchievementList.svelte";
  import ExportDialog from "./ExportDialog.svelte";
  import { blocks, forgetCard, loadBlocks } from "./blocks/store.svelte";
  import { openCompact } from "./compact";
  import MoreMenu from "./MoreMenu.svelte";
  import { appear, arrivals } from "./motion.svelte";
  import { chime } from "./sound.svelte";
  import RelockDialog from "./RelockDialog.svelte";
  import { on, send } from "./sync";
  import { loadGameTags } from "./tags.svelte";
  import type { GridPos, WorkspaceItem } from "./library";
  import ProgressRing from "./ProgressRing.svelte";
  import { openRelock, setPanelOpen, settings, ui } from "./settings.svelte";
  import Workspace from "./Workspace.svelte";

  let {
    game,
    imagesDir,
    onchanged,
    ondeleted,
    oncelebrate,
    onimport,
  }: {
    game: GameSummary;
    imagesDir: string;
    onchanged: (g: GameSummary) => void;
    ondeleted: (appid: number) => void;
    /** Rejouer la célébration du 100 % (jeu terminé). */
    oncelebrate: () => void;
    /** Importer un guide (choix du fichier, puis le jeu du guide s'ouvre). */
    onimport: () => void;
  } = $props();

  let achievements = $state<AchievementRow[]>([]);
  let refreshing = $state(false);
  let confirmDelete = $state(false);
  let exporting = $state(false);
  let error = $state("");
  let heroOk = $state(true);
  let workspace = $state.raw<WorkspaceItem[]>([]);
  let freeCards = $state<FreeCard[]>([]);
  const freeById = $derived(new Map(freeCards.map((f) => [f.id, f])));
  /** Carte libre qui vient d'être créée : son titre prend le focus. */
  let focusKey = $state<CardKey | null>(null);
  /** Succès cachés révélés (liste et cartes), oubliés en changeant de jeu. */
  const revealed = new SvelteSet<number>();

  const byId = $derived(new Map(achievements.map((a) => [a.id, a])));
  const personalOrder = $derived(
    new Map(workspace.filter((it) => it.achievement_id != null).map((it, i) => [it.achievement_id!, i])),
  );

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

  // Changement de jeu : la vue apparaît en douceur
  let view: HTMLElement | undefined = $state();
  $effect(() => {
    void appidKey;
    if (view) untrack(() => appear(view!));
  });

  // L'espace de travail ne dépend que du jeu (une actualisation n'y touche pas)
  $effect(() => {
    const appid = appidKey;
    revealed.clear();
    arrivals.clear();
    workspace = [];
    freeCards = [];
    focusKey = null;
    untrack(() => (ui.relock.open = false));
    loadBlocks(appid);
    loadGameTags(appid);
    cardSizes.clear();
    api.workspaceCardSizes(appid).then((sizes) => {
      if (appid !== game.appid) return;
      for (const s of sizes) if (!cardSizes.has(s.achievement_id)) cardSizes.set(s.achievement_id, { w: s.w, h: s.h });
    });
    api.freeCards(appid).then((f) => {
      if (appid === game.appid) freeCards = f;
    });
    api.workspaceList(appid).then((w) => {
      if (appid === game.appid) workspace = w;
    });
  });

  // Coche faite dans la fenêtre compacte
  $effect(() =>
    on("achievement-changed", ({ id, unlocked }) => {
      const row = achievements.find((x) => x.id === id);
      if (row) Object.assign(row, { unlocked, manual: true, unlock_time: unlocked ? (row.unlock_time ?? Math.floor(Date.now() / 1000)) : null });
    }),
  );

  // Enregistrements de l'espace mis en file, pour qu'ils arrivent dans l'ordre
  let saving: Promise<unknown> = Promise.resolve();
  function saveWorkspace(items: WorkspaceItem[]) {
    const appid = game.appid;
    workspace = items;
    for (const it of items) if (it.grid && it.achievement_id != null) cardSizes.set(it.achievement_id, { w: it.grid.w, h: it.grid.h });
    saving = saving
      .then(() => api.workspaceSave(appid, items))
      .then(() => send("data-changed", { appid }))
      .catch((e) => (error = `Enregistrement de l'espace de travail impossible : ${e}`));
  }

  /** Ajoute des succès à l'espace. `at` : case du tableau où le premier a été déposé. */
  function addToWorkspace(ids: number[], at?: { x: number; y: number }) {
    const present = new Set(workspace.map((it) => it.achievement_id));
    const fresh = ids.filter((id) => byId.has(id) && !present.has(id));
    if (fresh.length === 0) return;
    for (const id of fresh) arrivals.add(id);
    // Une carte déjà venue dans l'espace retrouve sa dernière taille ; elle s'agrandira de toute façon
    // si son contenu (ses blocs) ne tient pas dedans
    const size = (id: number) => cardSizes.get(id) ?? { w: 4, h: 8 };
    const place = (id: number, i: number): GridPos | null => (i === 0 && at ? { ...at, ...size(id) } : null);
    saveWorkspace([
      ...workspace,
      ...fresh.map((id, i) => ({
        achievement_id: id,
        card_id: null,
        column: "todo" as const,
        collapsed: false,
        grid: place(id, i),
        pinned: false,
        size: size(id),
        fresh: true,
      })),
    ]);
  }

  // ----- Cartes libres -----

  async function addFreeCard() {
    const appid = game.appid;
    try {
      const card = await api.freeCardCreate(appid, "");
      if (appid !== game.appid) return;
      freeCards.push(card);
      arrivals.add(-card.id);
      focusKey = -card.id;
      // Le focus ne vaut qu'à la création, pas quand la carte réapparaît (changement de vue)
      setTimeout(() => {
        if (focusKey === -card.id) focusKey = null;
      }, 1000);
      saveWorkspace([
        ...workspace,
        {
          achievement_id: null,
          card_id: card.id,
          column: "todo",
          collapsed: false,
          grid: null,
          pinned: false,
          size: { w: 4, h: 6 },
          fresh: true,
        },
      ]);
    } catch (e) {
      error = `Création de la carte impossible : ${e}`;
    }
  }

  async function renameFreeCard(id: number, title: string) {
    const card = freeCards.find((f) => f.id === id);
    if (!card) return;
    const before = card.title;
    card.title = title;
    try {
      await api.freeCardRename(id, title);
      send("data-changed", { appid: game.appid });
    } catch (e) {
      card.title = before;
      error = `Enregistrement du titre impossible : ${e}`;
    }
  }

  /** Appelé une fois la carte retirée de l'espace : supprime la carte et ses blocs. */
  async function deleteFreeCard(id: number) {
    try {
      await api.freeCardDelete(id);
      freeCards = freeCards.filter((f) => f.id !== id);
      forgetCard(-id);
      send("data-changed", { appid: game.appid });
    } catch (e) {
      error = `Suppression de la carte impossible : ${e}`;
    }
  }

  /** Re-lock confirmé par le joueur : décoche dans le logiciel, sans toucher aux guides. */
  async function relockDone() {
    const t = ui.relock.target;
    const appid = game.appid;
    if (t) {
      const row = achievements.find((x) => x.id === t.id);
      if (row?.unlocked) await toggle(row);
    } else {
      onchanged(await api.lockAll(appid));
      achievements = await api.achievements(appid);
      send("data-changed", { appid });
    }
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
    if (row.unlocked) chime();
    row.manual = true;
    // Comme le cœur Rust : une coche manuelle est datée d'aujourd'hui (si Steam ne l'avait pas datée)
    row.unlock_time = row.unlocked ? (row.unlock_time ?? Math.floor(Date.now() / 1000)) : null;
    try {
      onchanged(await api.setUnlocked(row.id, row.unlocked));
      send("data-changed", { appid: game.appid });
    } catch (e) {
      Object.assign(row, before);
      error = `Enregistrement impossible : ${e}`;
    }
  }
</script>

<section bind:this={view} class="flex h-screen min-w-0 flex-1 flex-col overflow-hidden">
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
          <span class="font-semibold {isComplete(game) ? 'text-gold' : ''}">{game.unlocked} / {game.total}</span>
          succès · {isComplete(game) ? 100 : pct} %
          <span class="text-muted">· importé depuis le {sourceLabel[game.source]}</span>
          {#if !game.has_api_names}<span class="text-muted">· certains ID techniques manquent</span>{/if}
        </p>
      </div>
      {#if isComplete(game)}
        <button
          onclick={oncelebrate}
          title="Revoir le récap du 100 %"
          class="gold-frame rounded-full bg-gold/15 px-3 py-1 text-sm font-bold text-gold backdrop-blur transition hover:bg-gold/25"
        >
          🏆 100 %
        </button>
      {:else}
        <ProgressRing value={pct} size={48} />
      {/if}
      <button
        onclick={() => openCompact(game.appid)}
        class="rounded-lg border border-line bg-panel/80 px-3 py-1.5 text-sm backdrop-blur transition hover:bg-raised"
        title="Petite fenêtre toujours au premier plan : cartes épinglées, compteurs et chronos"
      >
        Mode compact
      </button>
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
      <MoreMenu
        buttonClass="rounded-lg bg-panel/80 backdrop-blur"
        label="Plus d'actions sur le jeu"
        items={[
          {
            label: "Exporter le guide…",
            hint: "Fichier à garder en sauvegarde ou à envoyer à un ami",
            onclick: () => (exporting = true),
          },
          {
            label: "Importer un guide…",
            hint: "Ajoute les cartes et blocs d'un fichier de guide",
            onclick: onimport,
          },
          {
            label: "Réinitialiser tous les succès et statistiques du jeu dans Steam…",
            hint: "Prépare la commande console reset_all_stats, à lancer vous-même",
            onclick: () => openRelock(null),
          },
        ]}
      />
    </div>
  </header>

  {#if confirmDelete || error || blocks.error}
    <div class="space-y-2 px-6 pt-3">
      {#if confirmDelete}
        <div class="flex flex-wrap items-center gap-3 rounded-lg border border-danger/40 bg-danger/10 p-3 text-sm">
          <span class="flex-1">
            Supprimer « {game.name} » de la bibliothèque ? Ses succès, son espace de travail et ses guides seront effacés.
          </span>
          <button onclick={() => (confirmDelete = false)} class="rounded-md px-3 py-1 hover:bg-raised">Annuler</button>
          <button onclick={() => (exporting = true)} class="rounded-md border border-line px-3 py-1 hover:bg-raised">Exporter le guide d'abord…</button>
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
        {freeById}
        {focusKey}
        {revealed}
        {imagesDir}
        ontoggle={toggle}
        onchange={saveWorkspace}
        onadd={addToWorkspace}
        onaddfree={addFreeCard}
        onrename={renameFreeCard}
        ondeletefree={deleteFreeCard}
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

{#if exporting}
  <ExportDialog appid={game.appid} gameName={game.name} onclose={() => (exporting = false)} />
{/if}

{#if ui.relock.open}
  <RelockDialog
    appid={game.appid}
    gameName={game.name}
    target={ui.relock.target}
    onclose={() => (ui.relock.open = false)}
    onlock={relockDone}
  />
{/if}
