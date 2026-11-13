<script lang="ts">
  // Espace de travail d'un jeu : les succès choisis et les cartes libres, en vue Tableau (grille libre)
  // ou Liste. Chaque carte est repérée par sa `cardKey` (succès positif, carte libre négative).
  // - Tableau : gridstack (voir Board.svelte), position et taille de chaque carte enregistrées.
  // - Liste : pile de cartes réordonnée par svelte-dnd-action (souris, tactile, clavier).
  // Arrivée depuis le panneau par glisser natif (DRAG_MIME) dans les deux vues.
  import { flip } from "svelte/animate";
  import type { SvelteSet } from "svelte/reactivity";
  import { dragHandleZone, SHADOW_ITEM_MARKER_PROPERTY_NAME, type DndEvent } from "svelte-dnd-action";
  import Board from "./Board.svelte";
  import {
    cardKey,
    DRAG_MIME,
    isFreeKey,
    type AchievementRow,
    type CardKey,
    type FreeCard,
    type GridPos,
    type WorkspaceItem,
  } from "./library";
  import { setWorkspaceView, settings, ui } from "./settings.svelte";
  import { arrive, dur, leave } from "./motion.svelte";
  import WorkspaceCard from "./WorkspaceCard.svelte";

  /** Élément manipulé par svelte-dnd-action (qui exige un champ `id`). */
  type Card = { id: CardKey; item: WorkspaceItem; [SHADOW_ITEM_MARKER_PROPERTY_NAME]?: boolean };

  let {
    items,
    byId,
    freeById,
    focusKey,
    revealed,
    imagesDir,
    ontoggle,
    onchange,
    onadd,
    onaddfree,
    onrename,
    ondeletefree,
  }: {
    items: WorkspaceItem[];
    byId: Map<number, AchievementRow>;
    /** Cartes libres du jeu, par id. */
    freeById: Map<number, FreeCard>;
    /** Carte libre qui vient d'être créée (son titre prend le focus). */
    focusKey: CardKey | null;
    revealed: SvelteSet<number>;
    imagesDir: string;
    ontoggle: (a: AchievementRow) => void;
    onchange: (items: WorkspaceItem[]) => void;
    onadd: (ids: number[], at?: { x: number; y: number }) => void;
    /** Nouvelle carte libre. */
    onaddfree: () => void;
    onrename: (id: number, title: string) => void;
    /** Carte libre supprimée (après son retrait de l'espace). */
    ondeletefree: (id: number) => void;
  } = $props();

  const achievementCount = $derived(items.filter((it) => it.achievement_id != null).length);
  const freeCount = $derived(items.length - achievementCount);

  const FLIP = 150;

  // Vue liste : copie locale modifiée pendant un glisser, renvoyée au parent à la fin
  let list = $state.raw<Card[]>([]);
  let dragging = false;
  $effect(() => {
    const cards = items.map((item) => ({ id: cardKey(item), item }));
    if (!dragging) list = cards;
  });

  let justToggled = $state<number | null>(null);
  let board: ReturnType<typeof Board> | undefined = $state();
  let listHover = $state(false);

  function listConsider(e: CustomEvent<DndEvent<Card>>) {
    dragging = true;
    list = e.detail.items;
  }
  function listFinalize(e: CustomEvent<DndEvent<Card>>) {
    dragging = false;
    list = e.detail.items;
    onchange(list.map((c) => c.item));
  }

  // ----- Modifications d'une carte -----
  function update(key: CardKey, patch: Partial<WorkspaceItem>) {
    onchange(items.map((it) => (cardKey(it) === key ? { ...it, ...patch } : it)));
  }
  /** Cartes en train de partir (animation de sortie) : un 2ᵉ clic ne les retire pas deux fois. */
  const leaving = new Set<CardKey>();
  /** Retire une carte de l'espace ; une carte libre est supprimée avec ses blocs. */
  async function remove(key: CardKey) {
    if (leaving.has(key)) return;
    leaving.add(key);
    await leave(document.querySelector(`[data-card="${key}"]`));
    leaving.delete(key);
    // Tableau : les cartes du dessous remonteront dans la place libérée
    board?.willRemove(key);
    onchange(items.filter((it) => cardKey(it) !== key));
    if (isFreeKey(key)) ondeletefree(-key);
  }
  function togglePin(it: WorkspaceItem) {
    update(cardKey(it), { pinned: !it.pinned });
  }
  function toggle(a: AchievementRow) {
    justToggled = a.id;
    ontoggle(a);
  }
  function setAllCollapsed(collapsed: boolean) {
    onchange(items.map((it) => ({ ...it, collapsed })));
  }

  /** Tableau : nouvelles positions (n'émet que s'il y a un vrai changement). */
  function layout(positions: Map<CardKey, GridPos>) {
    let changed = false;
    const next = items.map((it) => {
      const p = positions.get(cardKey(it));
      const g = it.grid;
      if (!p || (g && g.x === p.x && g.y === p.y && g.w === p.w && g.h === p.h)) return it;
      changed = true;
      return { ...it, grid: p };
    });
    if (changed) onchange(next);
  }

  // ----- Vue liste : arrivée depuis le panneau -----
  function listDragOver(e: DragEvent) {
    if (!e.dataTransfer?.types.includes(DRAG_MIME)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    listHover = true;
  }
  function listDrop(e: DragEvent) {
    listHover = false;
    ui.draggingFromPanel = false;
    const raw = e.dataTransfer?.getData(DRAG_MIME);
    if (!raw) return;
    e.preventDefault();
    try {
      onadd((JSON.parse(raw) as unknown[]).filter((x): x is number => typeof x === "number"));
    } catch {
      /* données d'un autre glisser : ignorées */
    }
  }

  const spoiler = (a: AchievementRow) => settings.antiSpoiler && a.hidden && !a.unlocked && !revealed.has(a.id);
  const isShadow = (c: Card) => !!c[SHADOW_ITEM_MARKER_PROPERTY_NAME];
</script>

{#snippet card(it: WorkspaceItem, board: boolean)}
  {@const key = cardKey(it)}
  {@const a = it.achievement_id != null ? byId.get(it.achievement_id) : undefined}
  {@const free = it.card_id != null ? freeById.get(it.card_id) : undefined}
  {#if free}
    <WorkspaceCard
      {free}
      focusTitle={focusKey === key}
      column={it.column}
      collapsed={it.collapsed}
      pinned={it.pinned}
      spoiler={false}
      animate={false}
      {board}
      {imagesDir}
      ontoggle={() => {}}
      onreveal={() => {}}
      oncollapse={() => update(key, { collapsed: !it.collapsed })}
      onremove={() => remove(key)}
      oncolumn={(column) => update(key, { column })}
      onpin={() => togglePin(it)}
      onrename={(title) => onrename(free.id, title)}
    />
  {:else if a}
    <WorkspaceCard
      {a}
      column={it.column}
      collapsed={it.collapsed}
      pinned={it.pinned}
      spoiler={spoiler(a)}
      animate={justToggled === a.id}
      {board}
      {imagesDir}
      ontoggle={() => toggle(a)}
      onreveal={() => revealed.add(a.id)}
      oncollapse={() => update(a.id, { collapsed: !it.collapsed })}
      onremove={() => remove(a.id)}
      oncolumn={(column) => update(a.id, { column })}
      onpin={() => togglePin(it)}
    />
  {/if}
{/snippet}

{#snippet boardCard(it: WorkspaceItem)}{@render card(it, true)}{/snippet}

<div class="flex min-h-full flex-col">
  <!-- Barre de l'espace -->
  <div class="flex flex-wrap items-center gap-3 px-6 pt-4 pb-3">
    <h2 class="text-sm font-semibold tracking-wide text-muted uppercase">
      Espace de travail
      <span class="font-normal normal-case">
        · {achievementCount} succès{#if freeCount > 0}&nbsp;· {freeCount} carte{freeCount > 1 ? "s" : ""} libre{freeCount > 1 ? "s" : ""}{/if}
      </span>
    </h2>
    <button
      onclick={onaddfree}
      class="rounded-md border border-line px-2 py-0.5 text-xs text-muted transition hover:bg-raised hover:text-fg"
      title="Carte rattachée au jeu, pas à un succès : route générale, objets à ne pas rater, compteur commun…"
    >
      + Carte libre
    </button>
    {#if items.length > 0 && settings.workspaceView === "list"}
      <button onclick={() => setAllCollapsed(true)} class="text-xs text-muted underline hover:text-fg">tout replier</button>
      <button onclick={() => setAllCollapsed(false)} class="text-xs text-muted underline hover:text-fg">tout déplier</button>
    {/if}
    {#if items.length > 0 && settings.workspaceView === "board"}
      <span class="text-xs text-muted">Glissez une carte par son titre, tirez ses bords pour la redimensionner.</span>
      <button
        onclick={() => board?.packAll()}
        class="text-xs text-muted underline hover:text-fg"
        title="Remonte chaque carte au plus haut dans sa colonne, pour combler les trous">tout remonter</button
      >
    {/if}
    <div class="ml-auto inline-flex rounded-lg border border-line bg-panel p-0.5" role="group" aria-label="Vue de l'espace de travail">
      {#each [{ v: "board", l: "Tableau" }, { v: "list", l: "Liste" }] as const as o (o.v)}
        <button
          onclick={() => setWorkspaceView(o.v)}
          aria-pressed={settings.workspaceView === o.v}
          class="rounded-md px-3 py-1 text-sm transition
            {settings.workspaceView === o.v ? 'bg-raised font-medium text-fg' : 'text-muted hover:text-fg'}"
        >
          {o.l}
        </button>
      {/each}
    </div>
  </div>

  {#if items.length === 0}
    <div class="flex-1 px-6 pb-6" role="region" aria-label="Espace de travail vide" ondragover={listDragOver} ondragleave={() => (listHover = false)} ondrop={listDrop}>
      <div
        class="grid min-h-60 place-items-center rounded-xl border-2 border-dashed p-8 text-center text-sm transition
          {ui.draggingFromPanel || listHover ? 'border-accent bg-accent/5 text-fg' : 'border-line text-muted'}"
      >
        <div class="max-w-sm space-y-2">
          <p class="text-base font-medium text-fg">Choisissez les succès à travailler</p>
          <p>
            Glissez-les ici depuis la liste de droite, ou cochez leurs cases puis « Ajouter à l'espace ».
            Chacun deviendra une carte où ranger vos guides. Pour des notes qui concernent tout le jeu,
            créez une « carte libre ».
          </p>
        </div>
      </div>
    </div>
  {:else if settings.workspaceView === "board"}
    <!-- Vue tableau : grille libre -->
    <div class="flex flex-1 flex-col px-4 pb-6">
      <Board bind:this={board} {items} card={boardCard} onlayout={layout} ondropids={onadd} />
    </div>
  {:else}
    <!-- Vue liste : une seule pile de cartes -->
    <div class="flex-1 px-6 pb-6" role="region" aria-label="Cartes de l'espace de travail" ondragover={listDragOver} ondragleave={() => (listHover = false)} ondrop={listDrop}>
      <div
        use:dragHandleZone={{ items: list, flipDurationMs: dur(FLIP), type: "workspace", dropTargetStyle: {}, useCursorForDetection: true }}
        onconsider={listConsider}
        onfinalize={listFinalize}
        class="space-y-2 rounded-xl transition
          {ui.draggingFromPanel ? 'outline-2 outline-offset-4 outline-dashed' : ''}
          {listHover ? 'outline-accent' : 'outline-line'}"
      >
        {#each list as c (c.id)}
          <div
            animate:flip={{ duration: dur(FLIP) }}
            class={isShadow(c) ? "opacity-30" : ""}
            data-card={c.id}
            {@attach (el) => arrive(el, c.id)}
          >
            {@render card(c.item, false)}
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
