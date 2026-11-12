<script lang="ts">
  // Espace de travail d'un jeu : les succès choisis, en vue liste ou en colonnes.
  // Réordonnancement par svelte-dnd-action (souris, tactile, clavier) ;
  // arrivée depuis le panneau par glisser natif (DRAG_MIME).
  import { flip } from "svelte/animate";
  import type { SvelteSet } from "svelte/reactivity";
  import { dragHandleZone, SHADOW_ITEM_MARKER_PROPERTY_NAME, type DndEvent } from "svelte-dnd-action";
  import { columns, DRAG_MIME, type AchievementRow, type Column, type WorkspaceItem } from "./library";
  import { setWorkspaceView, settings, ui } from "./settings.svelte";
  import WorkspaceCard from "./WorkspaceCard.svelte";

  /** Élément manipulé par svelte-dnd-action (qui exige un champ `id`). */
  type Card = { id: number; column: Column; collapsed: boolean; [SHADOW_ITEM_MARKER_PROPERTY_NAME]?: boolean };

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
    onadd: (ids: number[], column: Column) => void;
  } = $props();

  const FLIP = 150;
  const zoneStyle = { outline: "2px dashed var(--color-accent)", "outline-offset": "4px", "border-radius": "12px" };

  const toCard = (it: WorkspaceItem): Card => ({ id: it.achievement_id, column: it.column, collapsed: it.collapsed });
  const toItem = (c: Card): WorkspaceItem => ({ achievement_id: c.id, column: c.column, collapsed: c.collapsed });

  // Copies locales, modifiées pendant un glisser puis renvoyées au parent à la fin
  let list = $state.raw<Card[]>([]);
  let cols = $state.raw<Record<Column, Card[]>>({ todo: [], doing: [], done: [] });
  let dragging = false;

  $effect(() => {
    const cards = items.map(toCard);
    if (dragging) return;
    list = cards;
    cols = {
      todo: cards.filter((c) => c.column === "todo"),
      doing: cards.filter((c) => c.column === "doing"),
      done: cards.filter((c) => c.column === "done"),
    };
  });

  let justToggled = $state<number | null>(null);
  let hoverZone = $state<Column | "list" | null>(null);

  // ----- Vue liste -----
  function listConsider(e: CustomEvent<DndEvent<Card>>) {
    dragging = true;
    list = e.detail.items;
  }
  function listFinalize(e: CustomEvent<DndEvent<Card>>) {
    dragging = false;
    list = e.detail.items;
    onchange(list.map(toItem));
  }

  // ----- Vue colonnes -----
  function colConsider(col: Column, e: CustomEvent<DndEvent<Card>>) {
    dragging = true;
    cols = { ...cols, [col]: e.detail.items };
  }
  function colFinalize(col: Column, e: CustomEvent<DndEvent<Card>>) {
    dragging = false;
    cols = { ...cols, [col]: e.detail.items.map((c) => ({ ...c, column: col })) };
    // Ordre global = colonnes mises bout à bout
    onchange([...cols.todo, ...cols.doing, ...cols.done].map(toItem));
  }

  // ----- Modifications d'une carte -----
  function update(id: number, patch: Partial<WorkspaceItem>) {
    onchange(items.map((it) => (it.achievement_id === id ? { ...it, ...patch } : it)));
  }
  function remove(id: number) {
    onchange(items.filter((it) => it.achievement_id !== id));
  }
  function toggle(a: AchievementRow) {
    justToggled = a.id;
    ontoggle(a);
  }
  function setAllCollapsed(collapsed: boolean) {
    onchange(items.map((it) => ({ ...it, collapsed })));
  }

  // ----- Arrivée depuis le panneau (glisser natif) -----
  function dragOver(e: DragEvent, zone: Column | "list") {
    if (!e.dataTransfer?.types.includes(DRAG_MIME)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    hoverZone = zone;
  }
  function drop(e: DragEvent, column: Column) {
    const raw = e.dataTransfer?.getData(DRAG_MIME);
    hoverZone = null;
    ui.draggingFromPanel = false;
    if (!raw) return;
    e.preventDefault();
    try {
      const ids = (JSON.parse(raw) as unknown[]).filter((x): x is number => typeof x === "number");
      onadd(ids, column);
    } catch {
      /* données d'un autre glisser : ignorées */
    }
  }

  const spoiler = (a: AchievementRow) => settings.antiSpoiler && a.hidden && !a.unlocked && !revealed.has(a.id);
  const isShadow = (c: Card) => !!c[SHADOW_ITEM_MARKER_PROPERTY_NAME];
</script>

{#snippet card(c: Card, compact: boolean)}
  {@const a = byId.get(c.id)}
  {#if a}
    <div class={isShadow(c) ? "opacity-30" : ""}>
      <WorkspaceCard
        {a}
        column={c.column}
        collapsed={c.collapsed}
        spoiler={spoiler(a)}
        animate={justToggled === a.id}
        {compact}
        {imagesDir}
        ontoggle={() => toggle(a)}
        onreveal={() => revealed.add(a.id)}
        oncollapse={() => update(a.id, { collapsed: !c.collapsed })}
        onremove={() => remove(a.id)}
        oncolumn={(column) => update(a.id, { column })}
      />
    </div>
  {/if}
{/snippet}

<div class="flex h-full flex-col">
  <!-- Barre de l'espace -->
  <div class="flex flex-wrap items-center gap-3 px-6 pt-4 pb-3">
    <h2 class="text-sm font-semibold tracking-wide text-muted uppercase">
      Espace de travail <span class="font-normal normal-case">· {items.length} succès</span>
    </h2>
    {#if items.length > 0}
      <button onclick={() => setAllCollapsed(true)} class="text-xs text-muted underline hover:text-fg">tout replier</button>
      <button onclick={() => setAllCollapsed(false)} class="text-xs text-muted underline hover:text-fg">tout déplier</button>
    {/if}
    <div class="ml-auto inline-flex rounded-lg border border-line bg-panel p-0.5" role="group" aria-label="Vue de l'espace de travail">
      {#each [{ v: "list", l: "Liste" }, { v: "columns", l: "Colonnes" }] as const as o (o.v)}
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

  {#if settings.workspaceView === "list"}
    <!-- Vue liste : une seule pile de cartes -->
    <div
      class="flex-1 px-6 pb-6"
      role="region"
      aria-label="Cartes de l'espace de travail"
      ondragover={(e) => dragOver(e, "list")}
      ondragleave={() => (hoverZone = null)}
      ondrop={(e) => drop(e, "todo")}
    >
      {#if items.length === 0}
        <div
          class="grid min-h-60 place-items-center rounded-xl border-2 border-dashed p-8 text-center text-sm transition
            {ui.draggingFromPanel ? 'border-accent bg-accent/5 text-fg' : 'border-line text-muted'}"
        >
          <div class="max-w-sm space-y-2">
            <p class="text-base font-medium text-fg">Choisissez les succès à travailler</p>
            <p>
              Glissez-les ici depuis la liste de droite, ou cochez leurs cases puis « Ajouter à l'espace ».
              Chacun deviendra une carte où ranger vos guides.
            </p>
          </div>
        </div>
      {:else}
        <div
          use:dragHandleZone={{ items: list, flipDurationMs: FLIP, type: "workspace", dropTargetStyle: {}, useCursorForDetection: true }}
          onconsider={listConsider}
          onfinalize={listFinalize}
          class="space-y-2 rounded-xl transition
            {ui.draggingFromPanel ? 'outline-2 outline-offset-4 outline-dashed' : ''}
            {hoverZone === 'list' ? 'outline-accent' : 'outline-line'}"
        >
          {#each list as c (c.id)}
            <div animate:flip={{ duration: FLIP }}>{@render card(c, false)}</div>
          {/each}
        </div>
      {/if}
    </div>
  {:else}
    <!-- Vue colonnes : à faire / en cours / fait -->
    <div class="flex flex-1 gap-4 overflow-x-auto px-6 pb-6">
      {#each columns as col (col.id)}
        <section
          class="flex w-72 min-w-64 shrink-0 flex-col rounded-xl border bg-panel/40 transition
            {hoverZone === col.id ? 'border-accent bg-accent/5' : ui.draggingFromPanel ? 'border-dashed border-accent/60' : 'border-line'}"
          aria-label={col.label}
          ondragover={(e) => dragOver(e, col.id)}
          ondragleave={() => (hoverZone = null)}
          ondrop={(e) => drop(e, col.id)}
        >
          <h3 class="flex items-center justify-between px-3 pt-3 pb-2 text-sm font-semibold">
            {col.label}
            <span class="rounded-full bg-raised px-2 text-xs font-normal text-muted">{cols[col.id].filter((c) => !isShadow(c)).length}</span>
          </h3>
          <div
            use:dragHandleZone={{ items: cols[col.id], flipDurationMs: FLIP, type: "workspace", dropTargetStyle: zoneStyle, useCursorForDetection: true }}
            onconsider={(e) => colConsider(col.id, e)}
            onfinalize={(e) => colFinalize(col.id, e)}
            class="min-h-32 flex-1 space-y-2 px-2 pb-3"
          >
            {#each cols[col.id] as c (c.id)}
              <div animate:flip={{ duration: FLIP }}>{@render card(c, true)}</div>
            {/each}
          </div>
        </section>
      {/each}
    </div>
  {/if}
</div>
