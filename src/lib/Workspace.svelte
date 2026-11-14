<script lang="ts">
  // Espace de travail d'un jeu : les succès choisis et les cartes libres, en vue Tableau (grille libre)
  // ou Liste. Chaque carte est repérée par sa `cardKey` (succès positif, carte libre négative).
  // - Tableau : gridstack (voir Board.svelte), position et taille de chaque carte enregistrées.
  // - Liste : pile de cartes réordonnée par svelte-dnd-action (souris, tactile, clavier).
  // Arrivée depuis le panneau par glisser natif (DRAG_MIME) dans les deux vues.
  //
  // Sections (facultatives) : groupes de cartes nommés, repliables, réordonnés en glissant leur poignée
  // (glisser natif, SECTION_MIME). Les cartes sans section sont dans la zone du haut. Chaque section a son
  // propre tableau (ou sa propre pile en vue Liste) ; une carte passe de l'un à l'autre en la glissant.
  // `items` est toujours rangé ainsi : zone du haut, puis chaque section dans l'ordre (mode compact).
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
    sectionColor,
    sectionColorLabel,
    SECTION_COLORS,
    type Section,
    type WorkspaceItem,
  } from "./library";
  import MoreMenu from "./MoreMenu.svelte";
  import { setWorkspaceView, settings, ui } from "./settings.svelte";
  import { arrive, dur, leave } from "./motion.svelte";
  import WorkspaceCard from "./WorkspaceCard.svelte";

  /** Élément manipulé par svelte-dnd-action (qui exige un champ `id`). */
  type Card = { id: CardKey; item: WorkspaceItem; [SHADOW_ITEM_MARKER_PROPERTY_NAME]?: boolean };
  /** Zone du haut (`section` null) ou section. */
  type Group = { key: string; section: Section | null; items: WorkspaceItem[] };

  let {
    items,
    sections,
    byId,
    freeById,
    focusKey,
    focusSection,
    revealed,
    imagesDir,
    ontoggle,
    onchange,
    onadd,
    onaddfree,
    onrename,
    ondeletefree,
    onaddsection,
    onsections,
    ondeletesection,
  }: {
    items: WorkspaceItem[];
    /** Sections du jeu, dans l'ordre. */
    sections: Section[];
    byId: Map<number, AchievementRow>;
    /** Cartes libres du jeu, par id. */
    freeById: Map<number, FreeCard>;
    /** Carte libre qui vient d'être créée (son titre prend le focus). */
    focusKey: CardKey | null;
    /** Section qui vient d'être créée (son titre prend le focus). */
    focusSection: number | null;
    revealed: SvelteSet<number>;
    imagesDir: string;
    ontoggle: (a: AchievementRow) => void;
    onchange: (items: WorkspaceItem[]) => void;
    /** Succès ajoutés : `at` case du tableau du premier, `section` section d'arrivée (null : zone du haut). */
    onadd: (ids: number[], at?: { x: number; y: number }, section?: number | null) => void;
    /** Nouvelle carte libre. */
    onaddfree: () => void;
    onrename: (id: number, title: string) => void;
    /** Carte libre supprimée (après son retrait de l'espace). */
    ondeletefree: (id: number) => void;
    onaddsection: () => void;
    /** Sections modifiées (titre, état replié, ordre). */
    onsections: (sections: Section[]) => void;
    /** Supprime la section ; `removeCards` : ses cartes quittent aussi l'espace, sinon elles passent en haut. */
    ondeletesection: (id: number, removeCards: boolean) => void;
  } = $props();

  const achievementCount = $derived(items.filter((it) => it.achievement_id != null).length);
  const freeCount = $derived(items.length - achievementCount);

  const FLIP = 150;
  const SECTION_MIME = "application/x-grimoire-section";

  // ----- Sections -----
  const known = $derived(new Set(sections.map((s) => s.id)));
  /** Section d'une carte (null : zone du haut, y compris si sa section n'existe plus). */
  const sectionOf = (it: WorkspaceItem) => (it.section != null && known.has(it.section) ? it.section : null);
  const groups: Group[] = $derived([
    { key: "top", section: null, items: items.filter((it) => sectionOf(it) == null) },
    ...sections.map((s) => ({ key: String(s.id), section: s, items: items.filter((it) => sectionOf(it) === s.id) })),
  ]);
  const hasSections = $derived(sections.length > 0);

  /** Range les cartes : zone du haut, puis les sections dans l'ordre (ordre relatif conservé). */
  function ordered(list: WorkspaceItem[]): WorkspaceItem[] {
    const ids = new Set(sections.map((s) => s.id));
    const of = (it: WorkspaceItem) => (it.section != null && ids.has(it.section) ? it.section : null);
    return [...list.filter((it) => of(it) == null), ...sections.flatMap((s) => list.filter((it) => of(it) === s.id))];
  }
  function emit(list: WorkspaceItem[]) {
    onchange(ordered(list));
  }

  function patchSection(id: number, patch: Partial<Section>) {
    onsections(sections.map((s) => (s.id === id ? { ...s, ...patch } : s)));
  }
  function renameSection(s: Section, input: HTMLInputElement) {
    const title = input.value.trim();
    if (title !== s.title) patchSection(s.id, { title });
  }
  function moveSection(id: number, to: number) {
    const from = sections.findIndex((s) => s.id === id);
    if (from < 0) return;
    const next = [...sections];
    const [s] = next.splice(from, 1);
    next.splice(Math.max(0, Math.min(next.length, to > from ? to - 1 : to)), 0, s);
    if (next.some((x, i) => x.id !== sections[i].id)) onsections(next);
  }
  function setAllSections(collapsed: boolean) {
    onsections(sections.map((s) => ({ ...s, collapsed })));
  }

  /** Section dont la suppression est demandée (question affichée sous son en-tête). */
  let deleting = $state<number | null>(null);
  function askDelete(g: Group) {
    if (!g.section) return;
    if (g.items.length === 0) ondeletesection(g.section.id, false);
    else deleting = g.section.id;
  }

  // Réordonner les sections : glisser natif par la poignée ⠿ de l'en-tête
  let sectionDrag = $state<number | null>(null);
  /** Rang où la section glissée arrivera (trait d'insertion). */
  let dropIndex = $state<number | null>(null);
  function sectionDragStart(e: DragEvent, s: Section) {
    if (!e.dataTransfer) return;
    e.dataTransfer.setData(SECTION_MIME, String(s.id));
    e.dataTransfer.effectAllowed = "move";
    const head = (e.currentTarget as HTMLElement).closest("[data-section-head]");
    if (head) e.dataTransfer.setDragImage(head, 16, 16);
    sectionDrag = s.id;
  }
  function sectionDragOver(e: DragEvent, index: number) {
    if (sectionDrag == null || !e.dataTransfer?.types.includes(SECTION_MIME)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    const box = (e.currentTarget as HTMLElement).getBoundingClientRect();
    // Moitié haute de la section : avant elle ; moitié basse : après (au plus 40 px pour une grande section)
    const half = Math.min(box.height / 2, 40);
    dropIndex = e.clientY < box.top + half ? index : e.clientY > box.bottom - half ? index + 1 : (dropIndex ?? index);
  }
  function sectionDrop(e: DragEvent) {
    if (sectionDrag == null || !e.dataTransfer?.types.includes(SECTION_MIME)) return;
    e.preventDefault();
    if (dropIndex != null) moveSection(sectionDrag, dropIndex);
    sectionDragEnd();
  }
  function sectionDragEnd() {
    sectionDrag = null;
    dropIndex = null;
  }

  /** Succès glissés depuis le panneau sur l'en-tête d'une section : ajoutés dans la section. */
  let headHover = $state<number | null>(null);
  function headDragOver(e: DragEvent, s: Section) {
    if (!e.dataTransfer?.types.includes(DRAG_MIME)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    headHover = s.id;
  }
  function headDrop(e: DragEvent, s: Section) {
    headHover = null;
    const ids = panelIds(e);
    if (ids) onadd(ids, undefined, s.id);
  }

  // ----- Zone survolée pendant qu'on y amène une carte : son cadre s'éclaire -----
  /** Une carte du tableau est en train d'être déplacée. */
  let cardDrag = $state(false);
  /** Zone (`top` ou id de section) sous le curseur pendant un déplacement de carte ou un glisser du panneau. */
  let dropZone = $state<string | null>(null);
  function zoneAt(x: number, y: number): string | null {
    for (const el of document.querySelectorAll<HTMLElement>("[data-zone]")) {
      const b = el.getBoundingClientRect();
      if (x >= b.left && x <= b.right && y >= b.top && y <= b.bottom) return el.dataset.zone ?? null;
    }
    return null;
  }
  $effect(() => {
    if (!cardDrag) return;
    const move = (e: MouseEvent) => (dropZone = zoneAt(e.clientX, e.clientY));
    document.addEventListener("mousemove", move, { capture: true, passive: true });
    return () => document.removeEventListener("mousemove", move, { capture: true });
  });
  /** Glisser depuis le panneau des succès : même éclairage, par les événements du glisser natif. */
  function zoneDragOver(e: DragEvent, key: string) {
    if (e.dataTransfer?.types.includes(DRAG_MIME)) dropZone = key;
  }
  // Fin du geste : plus rien d'éclairé
  $effect(() => {
    if (!ui.draggingFromPanel && !cardDrag) dropZone = null;
  });

  /** Section dont on choisit la couleur (nuancier affiché sous son en-tête). */
  let coloring = $state<number | null>(null);

  // ----- Vue liste : une pile par zone, copie locale modifiée pendant un glisser -----
  let lists = $state.raw<Record<string, Card[]>>({});
  let dragging = false;
  $effect(() => {
    const next = Object.fromEntries(groups.map((g) => [g.key, g.items.map((item) => ({ id: cardKey(item), item }))]));
    if (!dragging) lists = next;
  });

  let justToggled = $state<number | null>(null);
  /** Tableaux affichés, par zone. */
  const boards: Record<string, ReturnType<typeof Board> | undefined> = $state({});
  let listHover = $state<string | null>(null);

  function listConsider(g: Group, e: CustomEvent<DndEvent<Card>>) {
    dragging = true;
    lists = { ...lists, [g.key]: e.detail.items };
  }
  function listFinalize(g: Group, e: CustomEvent<DndEvent<Card>>) {
    dragging = false;
    lists = { ...lists, [g.key]: e.detail.items };
    emit(groups.flatMap((x) => (lists[x.key] ?? []).map((c) => ({ ...c.item, section: x.section?.id ?? null }))));
  }

  // ----- Modifications d'une carte -----
  function update(key: CardKey, patch: Partial<WorkspaceItem>) {
    emit(items.map((it) => (cardKey(it) === key ? { ...it, ...patch } : it)));
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
    const it = items.find((x) => cardKey(x) === key);
    if (it) boards[groupKey(sectionOf(it))]?.willRemove(key);
    emit(items.filter((x) => cardKey(x) !== key));
    if (isFreeKey(key)) ondeletefree(-key);
  }
  const groupKey = (section: number | null) => (section == null ? "top" : String(section));
  function togglePin(it: WorkspaceItem) {
    update(cardKey(it), { pinned: !it.pinned });
  }
  function toggle(a: AchievementRow) {
    justToggled = a.id;
    ontoggle(a);
  }
  function setAllCollapsed(collapsed: boolean) {
    emit(items.map((it) => ({ ...it, collapsed })));
  }
  /** Change une carte de section par son menu : une place libre lui sera trouvée, à sa taille. */
  function moveTo(it: WorkspaceItem, section: number | null) {
    const size = it.grid ? { w: it.grid.w, h: it.grid.h } : it.size;
    update(cardKey(it), { section, grid: null, size });
  }
  /** Entrées « Déplacer vers… » du menu d'une carte. */
  function movesFor(it: WorkspaceItem) {
    if (!hasSections) return [];
    const here = sectionOf(it);
    const out = sections
      .filter((s) => s.id !== here)
      .map((s) => ({ label: `Déplacer vers « ${s.title || "Section sans titre"} »`, onclick: () => moveTo(it, s.id) }));
    if (here != null) out.unshift({ label: "Sortir de la section", onclick: () => moveTo(it, null) });
    return out;
  }

  /** Tableau d'une zone : nouvelles positions (n'émet que s'il y a un vrai changement). */
  function layout(section: number | null, positions: Map<CardKey, GridPos>) {
    let changed = false;
    const next = items.map((it) => {
      if (sectionOf(it) !== section) return it;
      const p = positions.get(cardKey(it));
      const g = it.grid;
      if (!p || (g && g.x === p.x && g.y === p.y && g.w === p.w && g.h === p.h)) return it;
      changed = true;
      return { ...it, grid: p };
    });
    if (changed) emit(next);
  }
  /** Carte glissée d'un tableau à celui d'une autre zone. */
  function moveIn(section: number | null, key: CardKey, pos: GridPos) {
    emit(items.map((it) => (cardKey(it) === key ? { ...it, section, grid: pos } : it)));
  }

  // ----- Arrivée depuis le panneau (vue liste et en-têtes de section) -----
  function panelIds(e: DragEvent): number[] | null {
    ui.draggingFromPanel = false;
    const raw = e.dataTransfer?.getData(DRAG_MIME);
    if (!raw) return null;
    e.preventDefault();
    try {
      return (JSON.parse(raw) as unknown[]).filter((x): x is number => typeof x === "number");
    } catch {
      return null; // données d'un autre glisser : ignorées
    }
  }
  function listDragOver(e: DragEvent, key: string) {
    if (!e.dataTransfer?.types.includes(DRAG_MIME)) return;
    e.preventDefault();
    e.stopPropagation();
    e.dataTransfer.dropEffect = "copy";
    listHover = key;
  }
  function listDrop(e: DragEvent, section: number | null) {
    listHover = null;
    e.stopPropagation();
    const ids = panelIds(e);
    if (ids) onadd(ids, undefined, section);
  }

  const spoiler = (a: AchievementRow) => settings.antiSpoiler && a.hidden && !a.unlocked && !revealed.has(a.id);
  const isShadow = (c: Card) => !!c[SHADOW_ITEM_MARKER_PROPERTY_NAME];

  /** Bilan d'une section : succès obtenus / succès de la section, et cartes libres. */
  function tally(g: Group) {
    const achs = g.items.filter((it) => it.achievement_id != null);
    const done = achs.filter((it) => byId.get(it.achievement_id!)?.unlocked).length;
    return { total: achs.length, done, free: g.items.length - achs.length };
  }
</script>

{#snippet card(it: WorkspaceItem, board: boolean)}
  {@const key = cardKey(it)}
  {@const a = it.achievement_id != null ? byId.get(it.achievement_id) : undefined}
  {@const free = it.card_id != null ? freeById.get(it.card_id) : undefined}
  {#if free}
    <WorkspaceCard
      {free}
      focusTitle={focusKey === key}
      collapsed={it.collapsed}
      pinned={it.pinned}
      spoiler={false}
      animate={false}
      {board}
      {imagesDir}
      moves={movesFor(it)}
      ontoggle={() => {}}
      onreveal={() => {}}
      oncollapse={() => update(key, { collapsed: !it.collapsed })}
      onremove={() => remove(key)}
      onpin={() => togglePin(it)}
      onrename={(title) => onrename(free.id, title)}
    />
  {:else if a}
    <WorkspaceCard
      {a}
      collapsed={it.collapsed}
      pinned={it.pinned}
      spoiler={spoiler(a)}
      animate={justToggled === a.id}
      {board}
      {imagesDir}
      moves={movesFor(it)}
      ontoggle={() => toggle(a)}
      onreveal={() => revealed.add(a.id)}
      oncollapse={() => update(a.id, { collapsed: !it.collapsed })}
      onremove={() => remove(a.id)}
      onpin={() => togglePin(it)}
    />
  {/if}
{/snippet}

{#snippet boardCard(it: WorkspaceItem)}{@render card(it, true)}{/snippet}

<!-- Cartes d'une zone : un tableau, ou une pile en vue Liste -->
{#snippet zone(g: Group)}
  {@const section = g.section?.id ?? null}
  {#if settings.workspaceView === "board"}
    {@const top = section == null}
    <div class="flex flex-col {top && !hasSections ? 'flex-1' : ''}">
      {#key top && hasSections}
        <Board
          bind:this={boards[g.key]}
          items={g.items}
          card={boardCard}
          minRow={top && !hasSections ? 8 : 3}
          spare={top && !hasSections ? 10 : g.items.length ? 2 : 0}
          hint={top ? (hasSections ? "Hors section : glissez des cartes ici si besoin" : "") : "Section vide : glissez-y des cartes"}
          onlayout={(p) => layout(section, p)}
          onmovein={(key, pos) => moveIn(section, key, pos)}
          ondropids={(ids, at) => onadd(ids, at, section)}
          ondragcard={(d) => (cardDrag = d)}
          framed={hasSections}
        />
      {/key}
    </div>
  {:else}
    <div
      role="region"
      aria-label={g.section ? `Cartes de la section « ${g.section.title} »` : "Cartes hors section"}
      ondragover={(e) => listDragOver(e, g.key)}
      ondragleave={() => (listHover = null)}
      ondrop={(e) => listDrop(e, section)}
    >
      <div
        use:dragHandleZone={{
          items: lists[g.key] ?? [],
          flipDurationMs: dur(FLIP),
          type: "workspace",
          dropTargetStyle: {},
          useCursorForDetection: true,
        }}
        onconsider={(e) => listConsider(g, e)}
        onfinalize={(e) => listFinalize(g, e)}
        class="space-y-2 rounded-xl transition
          {(lists[g.key] ?? []).length === 0 ? 'min-h-14' : ''}
          {ui.draggingFromPanel && !hasSections ? 'outline-2 outline-offset-4 outline-dashed' : ''}
          {listHover === g.key ? 'outline-accent' : 'outline-line'}"
      >
        {#each lists[g.key] ?? [] as c (c.id)}
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
      {#if (lists[g.key] ?? []).length === 0}
        <p class="-mt-10 mb-4 text-center text-sm text-muted">
          {g.section ? "Section vide : glissez-y des cartes" : "Hors section : glissez des cartes ici si besoin"}
        </p>
      {/if}
    </div>
  {/if}
{/snippet}

<!-- En-tête d'une section : poignée, repli, titre, bilan, menu -->
{#snippet head(g: Group, s: Section, index: number)}
  {@const t = tally(g)}
  <div
    data-section-head
    role="group"
    aria-label={`Section « ${s.title || "sans titre"} »`}
    ondragover={(e) => headDragOver(e, s)}
    ondragleave={() => (headHover = null)}
    ondrop={(e) => headDrop(e, s)}
    class="section-head flex items-center gap-1.5 px-1.5 py-1 transition
      {s.collapsed && coloring !== s.id && deleting !== s.id ? 'rounded-b-[5px]' : 'border-b'}
      {headHover === s.id ? 'is-hover' : ''}"
  >
    <span
      draggable="true"
      ondragstart={(e) => sectionDragStart(e, s)}
      ondragend={sectionDragEnd}
      role="button"
      tabindex="-1"
      aria-label={`Déplacer la section « ${s.title} »`}
      title="Glisser pour déplacer la section"
      class="grid h-7 w-5 shrink-0 cursor-grab place-items-center rounded text-muted hover:bg-raised hover:text-fg active:cursor-grabbing"
      >⠿</span
    >
    <button
      onclick={() => patchSection(s.id, { collapsed: !s.collapsed })}
      aria-expanded={!s.collapsed}
      aria-label={s.collapsed ? "Déplier la section" : "Replier la section"}
      title={s.collapsed ? "Déplier" : "Replier"}
      class="grid h-7 w-7 shrink-0 place-items-center rounded text-muted transition hover:bg-raised hover:text-fg"
    >
      <span class="transition-transform {s.collapsed ? '-rotate-90' : ''}">▾</span>
    </button>
    <button
      onclick={() => (coloring = coloring === s.id ? null : s.id)}
      aria-label="Couleur de la section"
      aria-expanded={coloring === s.id}
      title="Couleur de la section"
      class="grid h-7 w-6 shrink-0 place-items-center rounded transition hover:bg-raised"
    >
      <span class="h-3 w-3 rounded-sm" style:background="var(--zone-color)"></span>
    </button>
    <input
      value={s.title}
      placeholder="Section sans titre"
      aria-label="Nom de la section"
      maxlength="200"
      onchange={(e) => renameSection(s, e.currentTarget)}
      onkeydown={(e) => {
        if (e.key === "Enter") e.currentTarget.blur();
        if (e.key === "Escape") {
          e.currentTarget.value = s.title;
          e.currentTarget.blur();
        }
      }}
      {@attach (el) => {
        if (focusSection === s.id) el.focus();
      }}
      class="min-w-0 flex-1 rounded-md border border-transparent bg-transparent px-1.5 py-0.5 font-semibold outline-none
        placeholder:font-normal placeholder:text-muted hover:border-line focus:border-accent focus:bg-bg"
    />
    <span class="shrink-0 text-xs text-muted tabular-nums">
      {#if t.total > 0}
        <span class={t.done === t.total ? "font-semibold text-ok" : ""}>{t.done} / {t.total}</span>
      {/if}
      {#if t.free > 0}<span>{t.total > 0 ? " · " : ""}{t.free} carte{t.free > 1 ? "s" : ""} libre{t.free > 1 ? "s" : ""}</span>{/if}
      {#if g.items.length === 0}<span>vide</span>{/if}
    </span>
    {#if t.total > 0}
      <span class="h-1.5 w-16 shrink-0 overflow-hidden rounded-sm bg-raised" aria-hidden="true">
        <span class="block h-full" style:width="{(100 * t.done) / t.total}%" style:background="var(--zone-color)"></span>
      </span>
    {/if}
    <MoreMenu
      class="shrink-0"
      label="Actions de la section"
      items={[
        ...(index > 0 ? [{ label: "Monter", onclick: () => moveSection(s.id, index - 1) }] : []),
        ...(index < sections.length - 1 ? [{ label: "Descendre", onclick: () => moveSection(s.id, index + 2) }] : []),
        { label: "Changer la couleur…", onclick: () => (coloring = s.id) },
        { label: "Supprimer la section…", onclick: () => askDelete(g) },
      ]}
    />
  </div>
  {#if coloring === s.id}
    <div class="section-band flex flex-wrap items-center gap-2 border-b px-3 py-2" role="radiogroup" aria-label="Couleur de la section">
      {#each SECTION_COLORS as c (c)}
        <button
          role="radio"
          aria-checked={s.color === c}
          aria-label={sectionColorLabel[c]}
          title={sectionColorLabel[c]}
          onclick={() => {
            patchSection(s.id, { color: c });
            coloring = null;
          }}
          class="h-6 w-6 rounded-sm transition hover:scale-110 {s.color === c ? 'ring-2 ring-fg ring-offset-2 ring-offset-panel' : ''}"
          style:background={sectionColor(c)}
        ></button>
      {/each}
    </div>
  {/if}
  {#if deleting === s.id}
    <div class="m-2 rounded border border-danger/40 bg-danger/10 p-3 text-sm">
      <p class="font-medium">
        Supprimer la section « {s.title || "sans titre"} » ? Elle contient {g.items.length} carte{g.items.length > 1 ? "s" : ""}.
      </p>
      <p class="mt-1 text-muted">
        Que faire de ses cartes ? Les guides des succès sont conservés dans tous les cas : ils reviennent si
        vous remettez le succès dans l'espace.{#if t.free > 0}{" "}Les cartes libres, elles, seraient supprimées
          avec leur contenu.{/if}
      </p>
      <div class="mt-2 flex flex-wrap justify-end gap-2">
        <button onclick={() => (deleting = null)} class="rounded-md px-2 py-0.5 hover:bg-raised">Annuler</button>
        <button
          onclick={() => {
            deleting = null;
            ondeletesection(s.id, false);
          }}
          class="rounded-md border border-line px-2 py-0.5 hover:bg-raised"
          title="Les cartes restent dans l'espace, au-dessus des sections">Garder les cartes</button
        >
        <button
          onclick={() => {
            deleting = null;
            ondeletesection(s.id, true);
          }}
          class="rounded-md bg-danger px-2 py-0.5 font-medium text-white">Retirer aussi les cartes</button
        >
      </div>
    </div>
  {/if}
{/snippet}

<div class="flex min-h-full flex-col {sectionDrag != null ? 'section-dragging' : ''}">
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
    <button
      onclick={onaddsection}
      class="rounded-md border border-line px-2 py-0.5 text-xs text-muted transition hover:bg-raised hover:text-fg"
      title="Groupe de cartes nommé et repliable : une phase du 100 %, les collectibles, le grind…"
    >
      + Section
    </button>
    {#if hasSections}
      <button onclick={() => setAllSections(true)} class="text-xs text-muted underline hover:text-fg">replier les sections</button>
      <button onclick={() => setAllSections(false)} class="text-xs text-muted underline hover:text-fg">déplier les sections</button>
    {/if}
    {#if items.length > 0 && settings.workspaceView === "list"}
      <button onclick={() => setAllCollapsed(true)} class="text-xs text-muted underline hover:text-fg">tout replier</button>
      <button onclick={() => setAllCollapsed(false)} class="text-xs text-muted underline hover:text-fg">tout déplier</button>
    {/if}
    {#if items.length > 0 && settings.workspaceView === "board"}
      <span class="text-xs text-muted">Glissez une carte par son titre, tirez ses bords pour la redimensionner.</span>
      <button
        onclick={() => Object.values(boards).forEach((b) => b?.packAll())}
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

  {#if items.length === 0 && !hasSections}
    <div
      class="flex-1 px-6 pb-6"
      role="region"
      aria-label="Espace de travail vide"
      ondragover={(e) => listDragOver(e, "top")}
      ondragleave={() => (listHover = null)}
      ondrop={(e) => listDrop(e, null)}
    >
      <div
        class="grid min-h-60 place-items-center rounded-xl border-2 border-dashed p-8 text-center text-sm transition
          {ui.draggingFromPanel || listHover ? 'border-accent bg-accent/5 text-fg' : 'border-line text-muted'}"
      >
        <div class="max-w-sm space-y-2">
          <p class="text-base font-medium text-fg">Choisissez les succès à travailler</p>
          <p>
            Glissez-les ici depuis la liste de droite, ou cochez leurs cases puis « Ajouter à l'espace ».
            Chacun deviendra une carte où ranger vos guides. Pour des notes qui concernent tout le jeu,
            créez une « carte libre » ; pour regrouper vos cartes (phases, collectibles…), créez des sections.
          </p>
        </div>
      </div>
    </div>
  {:else}
    <div class="flex flex-1 flex-col pb-6 {settings.workspaceView === 'board' ? 'px-4' : 'px-6'}">
      <!-- Zone du haut : cartes hors section (encadrée seulement quand il y a des sections) -->
      {#if hasSections}
        <div
          data-zone="top"
          role="group"
          aria-label="Cartes hors section"
          ondragover={(e) => zoneDragOver(e, "top")}
          class="zone-frame mb-3 {dropZone === 'top' ? 'is-target' : ''}"
          style:--zone-color="var(--color-muted)"
        >
          <div class="px-3 pt-1.5 text-xs font-medium tracking-wide text-muted uppercase">Hors section</div>
          <div class={settings.workspaceView === "board" ? "p-1.5" : "p-2.5"}>{@render zone(groups[0])}</div>
        </div>
      {:else}
        <div class="flex flex-1 flex-col">{@render zone(groups[0])}</div>
      {/if}

      <!-- Sections, dans l'ordre -->
      {#each groups.slice(1) as g, i (g.key)}
        {@const s = g.section!}
        <section
          aria-label={s.title || "Section sans titre"}
          data-zone={g.key}
          ondragover={(e) => {
            sectionDragOver(e, i);
            zoneDragOver(e, g.key);
          }}
          ondrop={sectionDrop}
          class="zone-frame relative mb-3 {sectionDrag === s.id ? 'opacity-50' : ''} {dropZone === g.key ? 'is-target' : ''}"
          style:--zone-color={sectionColor(s.color)}
        >
          {#if dropIndex === i && sectionDrag != null}
            <div class="drop-line -top-2"></div>
          {/if}
          {@render head(g, s, i)}
          {#if !s.collapsed}
            <div class={settings.workspaceView === "board" ? "p-1.5" : "p-2.5"}>{@render zone(g)}</div>
          {/if}
          {#if dropIndex === i + 1 && i === sections.length - 1 && sectionDrag != null}
            <div class="drop-line -bottom-2"></div>
          {/if}
        </section>
      {/each}
    </div>
  {/if}
</div>

<style>
  /* Pendant qu'une section est glissée, les cartes (et leurs vidéos) ne captent pas le glisser */
  .section-dragging :global(.grid-stack-item),
  .section-dragging :global([data-card]) {
    pointer-events: none;
  }
  /* Cadre d'une zone (section, ou « hors section ») dans la couleur de la section */
  .zone-frame {
    border: 1px solid color-mix(in srgb, var(--zone-color) 70%, transparent);
    background: color-mix(in srgb, var(--zone-color) 7%, var(--color-bg));
    border-radius: 6px;
    transition:
      border-color 0.15s,
      background-color 0.15s;
  }
  /* Zone où arrivera la carte en cours de glisser */
  .zone-frame.is-target {
    border-style: dashed;
    border-color: var(--zone-color);
    outline: 1px dashed var(--zone-color);
    background: color-mix(in srgb, var(--zone-color) 12%, var(--color-bg));
  }
  .section-head,
  .section-band {
    background: color-mix(in srgb, var(--zone-color) 24%, var(--color-panel));
    border-color: color-mix(in srgb, var(--zone-color) 55%, transparent);
  }
  .section-head {
    border-radius: 5px 5px 0 0;
  }
  .section-head.is-hover {
    background: color-mix(in srgb, var(--zone-color) 38%, var(--color-panel));
  }
  .drop-line {
    position: absolute;
    left: 0;
    right: 0;
    height: 3px;
    border-radius: 2px;
    background: var(--color-accent);
    pointer-events: none;
  }
</style>
