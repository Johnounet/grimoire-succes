<script lang="ts">
  import {
    api,
    heroUrl,
    isComplete,
    percent,
    sourceLabel,
    type AchievementRow,
    type CardKey,
    type FreeCard,
    type GameSummary,
    type Section,
    SECTION_COLORS,
  } from "./library";
  import { untrack } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import ExportDialog from "./ExportDialog.svelte";
  import { provideHelpGame } from "./helpSearch";
  import { blocks, forgetCard, loadBlocks } from "./blocks/store.svelte";
  import { openCompact } from "./compact";
  import MoreMenu from "./MoreMenu.svelte";
  import { appear } from "./motion.svelte";
  import { chime } from "./sound.svelte";
  import RelockDialog from "./RelockDialog.svelte";
  import { on, send } from "./sync";
  import { loadGameTags } from "./tags.svelte";
  import type { WorkspaceItem } from "./library";
  import ProgressRing from "./ProgressRing.svelte";
  import { openRelock, ui } from "./settings.svelte";
  import Workspace from "./Workspace.svelte";
  import NotesPanel from "./NotesPanel.svelte";

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

  // Boutons « Aide » des succès : nom et appid du jeu lus au clic
  provideHelpGame(() => game);

  let achievements = $state<AchievementRow[]>([]);
  let refreshing = $state(false);
  let confirmDelete = $state(false);
  let exporting = $state(false);
  let error = $state("");
  let heroOk = $state(true);
  let workspace = $state.raw<WorkspaceItem[]>([]);
  let freeCards = $state<FreeCard[]>([]);

  // ----- Onglets « Succès » / « Bloc-notes » : le dernier ouvert est retenu pour chaque jeu -----
  let tab = $state<"achievements" | "notes">("achievements");
  /** Jeux dont le dernier onglet ouvert est le bloc-notes (réglage `notes_tab`). */
  let notesGames: number[] = [];
  $effect(() => {
    const appid = game.appid;
    api
      .getSetting("notes_tab")
      .then((v) => {
        try {
          notesGames = v ? (JSON.parse(v) as number[]) : [];
        } catch {
          notesGames = [];
        }
        if (appid === game.appid) tab = notesGames.includes(appid) ? "notes" : "achievements";
      })
      .catch(() => {});
  });
  function setTab(t: "achievements" | "notes") {
    tab = t;
    const appid = game.appid;
    notesGames = notesGames.filter((a) => a !== appid);
    if (t === "notes") notesGames.push(appid);
    api.setSetting("notes_tab", JSON.stringify(notesGames)).catch(() => {});
  }
  const freeById = $derived(new Map(freeCards.map((f) => [f.id, f])));
  /** Préambule du jeu (carte libre `intro`), s'il existe. */
  const intro = $derived(freeCards.find((f) => f.intro) ?? null);
  /** Carte libre qui vient d'être créée : son titre prend le focus. */
  let focusKey = $state<CardKey | null>(null);
  let sections = $state.raw<Section[]>([]);
  /** Nombre de colonnes de la zone « Autres succès ». */
  let restColumns = $state(4);
  /** Section qui vient d'être créée : son titre prend le focus. */
  let focusSection = $state<number | null>(null);
  /** Succès cachés révélés (liste et cartes), oubliés en changeant de jeu. */
  const revealed = new SvelteSet<number>();

  const byId = $derived(new Map(achievements.map((a) => [a.id, a])));

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
    workspace = [];
    freeCards = [];
    sections = [];
    focusKey = null;
    focusSection = null;
    untrack(() => (ui.relock.open = false));
    loadBlocks(appid);
    loadGameTags(appid);
    api.freeCards(appid).then((f) => {
      if (appid === game.appid) freeCards = f;
    });
    // Sections (et colonnes) avant les cartes : une carte n'est rangée dans sa section que si celle-ci
    // est connue, et une carte pas encore placée va dans la colonne qui a le moins de cartes
    Promise.all([
      api.sections(appid).then((s) => {
        if (appid === game.appid) sections = s;
      }),
      api.restColumns(appid).then((n) => {
        if (appid === game.appid) restColumns = n;
      }),
    ])
      .finally(() =>
        api.workspaceList(appid).then((w) => {
          if (appid === game.appid) workspace = w;
        }),
      );
  });

  // Coche faite dans la fenêtre compacte
  $effect(() =>
    on("achievement-changed", ({ id, unlocked }) => {
      const row = achievements.find((x) => x.id === id);
      if (row) Object.assign(row, { unlocked, manual: true, unlock_time: unlocked ? (row.unlock_time ?? Math.floor(Date.now() / 1000)) : null });
    }),
  );
  $effect(() =>
    on("free-card-changed", ({ id, done }) => {
      const card = freeCards.find((f) => f.id === id);
      if (card) card.done = done;
    }),
  );

  // Enregistrements de l'espace mis en file, pour qu'ils arrivent dans l'ordre
  let saving: Promise<unknown> = Promise.resolve();
  function saveWorkspace(items: WorkspaceItem[]) {
    const appid = game.appid;
    workspace = items;
    saving = saving
      .then(() => api.workspaceSave(appid, items))
      .then(() => send("data-changed", { appid }))
      .catch((e) => (error = `Enregistrement de l'espace de travail impossible : ${e}`));
  }

  // ----- Sections -----

  /** Enregistre titre, état replié et ordre des sections (dans l'ordre des enregistrements de l'espace). */
  function saveSections(next: Section[]) {
    const appid = game.appid;
    sections = next;
    // L'espace est rangé sections dans l'ordre puis hors section : le suivre (mode compact, export)
    const rank = new Map(next.map((s, i) => [s.id, i]));
    const r = (section: number | null | undefined) => (section != null ? (rank.get(section) ?? next.length) : next.length);
    const sorted = workspace.map((it, i) => ({ it, i })).sort((a, b) => r(a.it.section) - r(b.it.section) || a.i - b.i);
    if (sorted.some((x, i) => x.i !== i)) saveWorkspace(sorted.map((x) => x.it));
    saving = saving
      .then(() => api.sectionsSave(appid, next))
      .catch((e) => (error = `Enregistrement des sections impossible : ${e}`));
  }

  function setRestColumns(n: number) {
    const appid = game.appid;
    restColumns = n;
    saving = saving
      .then(() => api.restColumnsSet(appid, n))
      .catch((e) => (error = `Enregistrement des colonnes impossible : ${e}`));
  }

  async function addSection() {
    const appid = game.appid;
    try {
      // Couleur suivante de la palette, la première pas encore prise de préférence
      const used = new Set(sections.map((x) => x.color));
      const color = SECTION_COLORS.find((c) => !used.has(c)) ?? SECTION_COLORS[sections.length % SECTION_COLORS.length];
      const s = await api.sectionCreate(appid, "", color);
      if (appid !== game.appid) return;
      sections = [...sections, s];
      focusSection = s.id;
      setTimeout(() => {
        if (focusSection === s.id) focusSection = null;
      }, 1000);
    } catch (e) {
      error = `Création de la section impossible : ${e}`;
    }
  }

  /** Supprime une section : ses cartes passent hors section (« Autres succès »), à la fin, avec leurs guides. */
  function deleteSection(id: number) {
    const appid = game.appid;
    const inside = workspace.filter((it) => it.section === id);
    if (inside.length) saveWorkspace([...workspace.filter((it) => it.section !== id), ...inside.map((it) => ({ ...it, section: null }))]);
    sections = sections.filter((s) => s.id !== id);
    saving = saving
      .then(() => api.sectionDelete(id))
      .then(() => send("data-changed", { appid }))
      .catch((e) => (error = `Suppression de la section impossible : ${e}`));
  }

  // ----- Cartes libres -----

  async function addFreeCard() {
    const appid = game.appid;
    try {
      const card = await api.freeCardCreate(appid, "");
      if (appid !== game.appid) return;
      freeCards.push(card);
      focusKey = -card.id;
      // Le focus ne vaut qu'à la création
      setTimeout(() => {
        if (focusKey === -card.id) focusKey = null;
      }, 1000);
      // En tête de la première section (ou de tout, sans section) : visible sans chercher
      const section = sections[0]?.id ?? null;
      const at = Math.max(0, workspace.findIndex((it) => (it.section ?? null) === section));
      const next = [...workspace];
      next.splice(at, 0, { achievement_id: null, card_id: card.id, collapsed: false, grid: null, pinned: false, section, col: 0 });
      saveWorkspace(next);
    } catch (e) {
      error = `Création de la carte impossible : ${e}`;
    }
  }

  // ----- Préambule -----

  async function addIntro() {
    const appid = game.appid;
    try {
      const card = await api.introCreate(appid);
      if (appid !== game.appid) return;
      if (!freeCards.some((f) => f.id === card.id)) freeCards.push(card);
      focusKey = -card.id;
      setTimeout(() => {
        if (focusKey === -card.id) focusKey = null;
      }, 1000);
    } catch (e) {
      error = `Création du préambule impossible : ${e}`;
    }
  }

  async function collapseIntro(collapsed: boolean) {
    const card = freeCards.find((f) => f.intro);
    if (!card) return;
    card.collapsed = collapsed;
    try {
      await api.freeCardSetCollapsed(card.id, collapsed);
    } catch (e) {
      card.collapsed = !collapsed;
      error = `Enregistrement impossible : ${e}`;
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

  async function toggleFreeCard(id: number) {
    const card = freeCards.find((f) => f.id === id);
    if (!card) return;
    card.done = !card.done;
    if (card.done) chime();
    try {
      await api.freeCardSetDone(id, card.done);
      send("free-card-changed", { id, done: card.done });
    } catch (e) {
      card.done = !card.done;
      error = `Enregistrement impossible : ${e}`;
    }
  }

  /** Supprime une carte libre et ses blocs. */
  async function deleteFreeCard(id: number) {
    saveWorkspace(workspace.filter((it) => it.card_id !== id));
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

  <div class="flex gap-1 border-b border-line px-6" role="tablist" aria-label="Contenu du jeu">
    {#each [{ v: "achievements", l: "Succès" }, { v: "notes", l: "Bloc-notes" }] as const as t (t.v)}
      <button
        role="tab"
        aria-selected={tab === t.v}
        onclick={() => setTab(t.v)}
        class="-mb-px border-b-2 px-3 py-2 text-sm transition
          {tab === t.v ? 'border-accent font-medium text-fg' : 'border-transparent text-muted hover:text-fg'}"
      >
        {t.l}
      </button>
    {/each}
  </div>

  {#if tab === "notes"}
    <div class="min-h-0 flex-1 overflow-y-auto" role="tabpanel" aria-label="Bloc-notes">
      {#key game.appid}<NotesPanel appid={game.appid} />{/key}
    </div>
  {/if}

  <!-- Vue unique : tous les succès et les cartes libres. Gardée (cachée) pendant le bloc-notes : les vidéos
       ouvertes continuent et rien n'est à recharger au retour. -->
  <div class="min-h-0 flex-1 overflow-y-auto {tab === 'notes' ? 'hidden' : ''}" role="tabpanel" aria-label="Succès">
    {#key game.appid}
      <Workspace
        {achievements}
        items={workspace}
        {sections}
        {focusSection}
        {restColumns}
        onrestcolumns={setRestColumns}
        {intro}
        onaddintro={addIntro}
        onintrocollapse={collapseIntro}
        onaddsection={addSection}
        onsections={saveSections}
        ondeletesection={deleteSection}
        {byId}
        {freeById}
        {focusKey}
        {revealed}
        {imagesDir}
        ontoggle={toggle}
        onchange={saveWorkspace}
        onaddfree={addFreeCard}
        onrename={renameFreeCard}
        ontogglefree={toggleFreeCard}
        ondeletefree={deleteFreeCard}
      />
    {/key}
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
