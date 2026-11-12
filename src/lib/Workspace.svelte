<script lang="ts">
  // Espace de travail d'un jeu : les succès choisis, en vue Tableau (grille libre) ou Liste.
  // - Tableau : gridstack (voir Board.svelte), position et taille de chaque carte enregistrées.
  // - Liste : pile de cartes réordonnée par svelte-dnd-action (souris, tactile, clavier).
  // Arrivée depuis le panneau par glisser natif (DRAG_MIME) dans les deux vues.
  import { flip } from "svelte/animate";
  import type { SvelteSet } from "svelte/reactivity";
  import { dragHandleZone, SHADOW_ITEM_MARKER_PROPERTY_NAME, type DndEvent } from "svelte-dnd-action";
  import Board from "./Board.svelte";
  import { DRAG_MIME, type AchievementRow, type GridPos, type WorkspaceItem } from "./library";
  import { setWorkspaceView, settings, ui } from "./settings.svelte";
  import WorkspaceCard from "./WorkspaceCard.svelte";

  /** Élément manipulé par svelte-dnd-action (qui exige un champ `id`). */
  type Card = { id: number; item: WorkspaceItem; [SHADOW_ITEM_MARKER_PROPERTY_NAME]?: boolean };

  let {
    items,
    byId,
    revealed,
    imagesDir,
    ontoggle,
    onchange,
    onadd,
  }: {
    items: WorkspaceItem[];
    byId: Map<number, AchievementRow>;
    revealed: SvelteSet<number>;
    imagesDir: string;
    ontoggle: (a: AchievementRow) => void;
    onchange: (items: WorkspaceItem[]) => void;
    onadd: (ids: number[], at?: { x: number; y: number }) => void;
  } = $props();

  const FLIP = 150;

  // Vue liste : copie locale modifiée pendant un glisser, renvoyée au parent à la fin
  let list = $state.raw<Card[]>([]);
  let dragging = false;
  $effect(() => {
    const cards = items.map((item) => ({ id: item.achievement_id, item }));
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
  function update(id: number, patch: Partial<WorkspaceItem>) {
    onchange(items.map((it) => (it.achievement_id === id ? { ...it, ...patch } : it)));
  }
  function remove(id: number) {
    // Tableau : les cartes du dessous remonteront dans la place libérée
    board?.willRemove(id);
    onchange(items.filter((it) => it.achievement_id !== id));
  }
  function toggle(a: AchievementRow) {
    justToggled = a.id;
    ontoggle(a);
  }
  function setAllCollapsed(collapsed: boolean) {
    onchange(items.map((it) => ({ ...it, collapsed })));
  }

  /** Tableau : nouvelles positions (n'émet que s'il y a un vrai changement). */
  function layout(positions: Map<number, GridPos>) {
    let changed = false;
    const next = items.map((it) => {
      const p = positions.get(it.achievement_id);
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
  {@const a = byId.get(it.achievement_id)}
  {#if a}
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
      onpin={() => update(a.id, { pinned: !it.pinned })}
    />
  {/if}
{/snippet}

{#snippet boardCard(it: WorkspaceItem)}{@render card(it, true)}{/snippet}

<div class="flex min-h-full flex-col">
  <!-- Barre de l'espace -->
  <div class="flex flex-wrap items-center gap-3 px-6 pt-4 pb-3">
    <h2 class="text-sm font-semibold tracking-wide text-muted uppercase">
      Espace de travail <span class="font-normal normal-case">· {items.length} succès</span>
    </h2>
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
            Chacun deviendra une carte où ranger vos guides.
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
        use:dragHandleZone={{ items: list, flipDurationMs: FLIP, type: "workspace", dropTargetStyle: {}, useCursorForDetection: true }}
        onconsider={listConsider}
        onfinalize={listFinalize}
        class="space-y-2 rounded-xl transition
          {ui.draggingFromPanel ? 'outline-2 outline-offset-4 outline-dashed' : ''}
          {listHover ? 'outline-accent' : 'outline-line'}"
      >
        {#each list as c (c.id)}
          <div animate:flip={{ duration: FLIP }} class={isShadow(c) ? "opacity-30" : ""}>{@render card(c.item, false)}</div>
        {/each}
      </div>
    </div>
  {/if}
</div>
