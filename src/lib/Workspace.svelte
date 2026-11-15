<script lang="ts">
  // Vue des succès d'un jeu (vue unique, 2026-11-15) : tous les succès et les cartes libres, rangés par
  // sections puis « Autres succès », en colonnes automatiques (Columns.svelte). Recherche, filtres et tri
  // en haut. Un succès sans guide est une ligne compacte, une carte complète dès son premier bloc.
  // Chaque carte est repérée par sa `cardKey` (succès positif, carte libre négative).
  //
  // Colonnes choisies par le joueur (2026-11-15) : chaque section (et « Autres succès ») a son nombre de
  // colonnes, chaque carte sa colonne (`col`). « Mon ordre » = celui de `items`, toujours rangé sections dans
  // l'ordre puis hors section, et dans chaque groupe colonne par colonne (mode compact, export). Une carte
  // pas encore placée (`col` absent : nouveau succès…) va dans la colonne qui a le moins de cartes.
  // Déplacement par clics : ⇅ sur une carte, puis clic sur la moitié haute (au-dessus) ou basse (en
  // dessous) d'une autre carte, ou sur « Mettre ici » en bas d'une colonne.
  //
  // Succès rattachés (`parent`) : un succès obtenu forcément en passant par un autre s'affiche dans la
  // carte de celui-ci, plus dans les colonnes. Un seul niveau (`nesting`) ; sa section suit celle du parent.
  import { tick, untrack } from "svelte";
  import type { SvelteSet } from "svelte/reactivity";
  import { blocksOf } from "./blocks/store.svelte";
  import Columns from "./Columns.svelte";
  import {
    apply,
    counts,
    defaultFilters,
    matches,
    rarityTiers,
    sortOptions,
    type Filters,
    type StateFilter,
  } from "./filters";
  import {
    cardKey,
    nesting,
    type AchievementRow,
    type CardKey,
    type FreeCard,
    sectionColor,
    sectionColorLabel,
    MAX_COLUMNS,
    SECTION_COLORS,
    type Section,
    type WorkspaceItem,
  } from "./library";
  import MoreMenu, { type MenuItem } from "./MoreMenu.svelte";
  import NestedAchievement from "./NestedAchievement.svelte";
  import { setAntiSpoiler, setStateFilter, settings } from "./settings.svelte";
  import { tags } from "./tags.svelte";
  import WorkspaceCard from "./WorkspaceCard.svelte";

  /** Section, ou « Autres » (`section` null). */
  type Group = { key: string; section: Section | null; items: WorkspaceItem[] };

  let {
    achievements,
    items,
    sections,
    byId,
    freeById,
    focusKey,
    focusSection,
    restColumns,
    onrestcolumns,
    revealed,
    imagesDir,
    ontoggle,
    onchange,
    onaddfree,
    onrename,
    ontogglefree,
    ondeletefree,
    onaddsection,
    onsections,
    ondeletesection,
  }: {
    /** Tous les succès du jeu, dans l'ordre Steam. */
    achievements: AchievementRow[];
    /** Toutes les cartes (succès et cartes libres), dans « Mon ordre ». */
    items: WorkspaceItem[];
    /** Sections du jeu, dans l'ordre. */
    sections: Section[];
    byId: Map<number, AchievementRow>;
    freeById: Map<number, FreeCard>;
    /** Carte libre qui vient d'être créée (son titre prend le focus). */
    focusKey: CardKey | null;
    /** Section qui vient d'être créée (son titre prend le focus). */
    focusSection: number | null;
    /** Nombre de colonnes de la zone « Autres succès ». */
    restColumns: number;
    onrestcolumns: (n: number) => void;
    revealed: SvelteSet<number>;
    imagesDir: string;
    ontoggle: (a: AchievementRow) => void;
    onchange: (items: WorkspaceItem[]) => void;
    onaddfree: () => void;
    onrename: (id: number, title: string) => void;
    ontogglefree: (id: number) => void;
    /** Carte libre supprimée (avec ses blocs). */
    ondeletefree: (id: number) => void;
    onaddsection: () => void;
    /** Sections modifiées (titre, état replié, couleur, colonnes, ordre). */
    onsections: (sections: Section[]) => void;
    /** Supprime la section ; ses cartes passent dans « Autres succès ». */
    ondeletesection: (id: number) => void;
  } = $props();

  // ----- Filtres et tri -----
  let filters = $state<Filters>({ ...defaultFilters(), sort: "personal" });
  // Filtre d'état : retenu d'un jeu et d'un lancement à l'autre
  $effect.pre(() => {
    filters.state = settings.stateFilter;
  });
  function setState(s: StateFilter) {
    filters.state = s;
    setStateFilter(s);
  }
  /** Seulement les succès qui ont un guide (et les cartes libres qui ont des blocs). */
  let guidedOnly = $state(false);
  // Étiquette filtrée supprimée entre-temps : on revient à « toutes »
  $effect(() => {
    const t = filters.tag;
    if (typeof t === "number" && !tags.list.some((x) => x.id === t)) filters.tag = "all";
  });
  const n = $derived(counts(achievements));
  const stateTabs: { value: StateFilter; label: string; count: () => number }[] = [
    { value: "locked", label: "À obtenir", count: () => n.locked },
    { value: "unlocked", label: "Obtenus", count: () => n.unlocked },
    { value: "all", label: "Tous", count: () => n.all },
  ];
  const filtered = $derived(
    filters.text.trim() !== "" || filters.rarity !== "all" || filters.tag !== "all" || guidedOnly,
  );
  function resetFilters() {
    filters.text = "";
    filters.rarity = "all";
    filters.tag = "all";
    guidedOnly = false;
  }

  const fold = (s: string) => s.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();

  /** La carte passe les filtres. Une carte libre n'a ni rareté ni étiquette : ces filtres la masquent. */
  function visible(it: WorkspaceItem): boolean {
    const key = cardKey(it);
    if (guidedOnly && blocksOf(key).length === 0) return false;
    if (it.card_id != null) {
      const f = freeById.get(it.card_id);
      if (!f || filters.rarity !== "all" || filters.tag !== "all") return false;
      if (filters.state === "locked" && f.done) return false;
      if (filters.state === "unlocked" && !f.done) return false;
      const q = fold(filters.text.trim());
      return !q || q.split(/\s+/).every((w) => fold(f.title).includes(w));
    }
    const a = it.achievement_id != null ? byId.get(it.achievement_id) : undefined;
    if (!a) return false;
    // Succès retiré de Steam : montré seulement s'il garde un guide
    if (a.removed && blocksOf(a.id).length === 0) return false;
    return matches(a, filters, tags.byAch);
  }

  // ----- Succès rattachés -----
  /** Succès rattaché → succès parent (rattachements valides seulement). */
  const nest = $derived(nesting(items));
  const parentOf = (it: WorkspaceItem) => (it.card_id == null && it.achievement_id != null ? nest.get(it.achievement_id) : undefined);
  const itemOfAch = $derived(new Map(items.filter((it) => it.card_id == null && it.achievement_id != null).map((it) => [it.achievement_id!, it])));
  /** Succès rattachés à chaque parent, dans « Mon ordre ». */
  const kidsOf = $derived.by(() => {
    const m = new Map<number, WorkspaceItem[]>();
    for (const it of items) {
      const p = parentOf(it);
      if (p != null) m.set(p, [...(m.get(p) ?? []), it]);
    }
    return m;
  });
  /** Lignes affichées dans la carte d'un parent : ses succès rattachés qui passent le filtre d'état
   *  (« À obtenir » cache ceux déjà obtenus, demande de l'utilisateur), sauf un succès retiré de Steam sans
   *  guide. Les autres filtres (recherche, rareté, étiquette) ne cachent pas les lignes : ils décident
   *  seulement si la carte du parent est montrée. */
  function kidRows(id: number) {
    return (kidsOf.get(id) ?? []).flatMap((it) => {
      const a = byId.get(it.achievement_id!);
      if (!a || (a.removed && blocksOf(a.id).length === 0)) return [];
      if ((filters.state === "locked" && a.unlocked) || (filters.state === "unlocked" && !a.unlocked)) return [];
      return [{ it, a }];
    });
  }

  // ----- Sections et groupes -----
  const known = $derived(new Set(sections.map((s) => s.id)));
  /** Section d'une carte (null : hors section, y compris si sa section n'existe plus). Un succès
   *  rattaché est dans la section de son parent. */
  function sectionOf(it: WorkspaceItem): number | null {
    const p = parentOf(it);
    const s = p != null ? itemOfAch.get(p)?.section : it.section;
    return s != null && known.has(s) ? s : null;
  }
  const groups: Group[] = $derived([
    ...sections.map((s) => ({ key: String(s.id), section: s, items: items.filter((it) => sectionOf(it) === s.id) })),
    { key: "rest", section: null, items: items.filter((it) => sectionOf(it) == null) },
  ]);
  const hasSections = $derived(sections.length > 0);

  /** Cartes affichées d'un groupe, dans l'ordre du tri choisi. */
  function shownOf(g: Group): WorkspaceItem[] {
    // Un parent passe les filtres si lui ou l'un de ses succès rattachés les passe
    const list = g.items.filter(
      (it) => parentOf(it) == null && (visible(it) || (kidsOf.get(it.achievement_id ?? 0) ?? []).some(visible)),
    );
    if (filters.sort === "personal") return list;
    // Autres tris : les cartes libres en tête, puis les succès triés (ordre Steam d'origine à égalité)
    const free = list.filter((it) => it.card_id != null);
    const inGroup = new Set(list.map((it) => it.achievement_id));
    const rows = apply(
      achievements.filter((a) => inGroup.has(a.id)),
      { ...filters, text: "", state: "all", rarity: "all", tag: "all" },
    );
    const byAch = new Map(list.map((it) => [it.achievement_id, it]));
    return [...free, ...rows.map((a) => byAch.get(a.id)!)];
  }
  const shown = $derived(groups.map((g) => ({ g, list: shownOf(g) })));

  // ----- Colonnes -----
  /** Nombre de colonnes d'un groupe. */
  const columnsOf = (g: Group) => Math.max(1, Math.min(MAX_COLUMNS, g.section ? (g.section.columns ?? 4) : restColumns));
  /** Colonne enregistrée d'une carte, bornée au nombre de colonnes (pas encore placée : 0, en attendant). */
  const colIn = (it: WorkspaceItem, cols: number) => Math.max(0, Math.min(cols - 1, it.col ?? 0));
  /** Colonne affichée de chaque carte d'un groupe : la sienne en « Mon ordre » ; avec un autre tri, les cartes
   *  sont réparties de gauche à droite (la colonne enregistrée n'est pas touchée). */
  function colMap(list: WorkspaceItem[], cols: number): Map<CardKey, number> {
    return new Map(list.map((it, i) => [cardKey(it), filters.sort === "personal" ? colIn(it, cols) : i % cols]));
  }

  /** Cartes pas encore placées (`col` absent) : chacune dans la colonne de son groupe qui a le moins de
   *  cartes (la plus à gauche à égalité), dans l'ordre. Liste modifiée, ou null s'il n'y avait rien à placer. */
  function placeUnplaced(list: WorkspaceItem[]): WorkspaceItem[] | null {
    const top = (it: WorkspaceItem) => parentOf(it) == null;
    if (!list.some((it) => top(it) && it.col == null)) return null;
    const out = [...list];
    for (const g of groups) {
      const cols = columnsOf(g);
      const count = Array<number>(cols).fill(0);
      for (const it of g.items) if (top(it) && it.col != null) count[colIn(it, cols)]++;
      for (const it of g.items) {
        if (!top(it) || it.col != null) continue;
        const c = count.indexOf(Math.min(...count));
        count[c]++;
        const i = out.indexOf(it);
        if (i >= 0) out[i] = { ...it, col: c };
      }
    }
    return out;
  }
  // Au premier affichage (cartes rangées avant les colonnes), puis pour chaque nouvelle carte
  $effect(() => {
    const next = placeUnplaced(items);
    if (next) untrack(() => emit(next));
  });

  /** Change le nombre de colonnes d'un groupe. Les cartes des colonnes supprimées vont en bas de la dernière
   *  colonne restante, dans leur ordre ; les nouvelles colonnes arrivent vides. */
  function setColumns(g: Group, cols: number) {
    cols = Math.max(1, Math.min(MAX_COLUMNS, cols));
    if (g.section) patchSection(g.section.id, { columns: cols });
    else onrestcolumns(cols);
    const out = g.items.filter((it) => parentOf(it) == null && (it.col ?? 0) >= cols);
    if (out.length) emit([...items.filter((it) => !out.includes(it)), ...out.map((it) => ({ ...it, col: cols - 1 }))]);
  }
  const shownCount = $derived(shown.reduce((s, x) => s + x.list.length, 0));

  /** Bilan d'un groupe : succès obtenus / succès (hors retirés de Steam), et cartes libres. */
  function tally(g: Group) {
    const achs = g.items.flatMap((it) => {
      const a = it.achievement_id != null ? byId.get(it.achievement_id) : undefined;
      return a && !a.removed ? [a] : [];
    });
    const done = achs.filter((a) => a.unlocked).length;
    return { total: achs.length, done, free: g.items.filter((it) => it.card_id != null).length };
  }

  /** Range les cartes : sections dans l'ordre, puis hors section ; dans chaque groupe, colonne par colonne
   *  (ordre relatif conservé dans une colonne). */
  function ordered(list: WorkspaceItem[]): WorkspaceItem[] {
    const of = (it: WorkspaceItem) => (it.section != null && known.has(it.section) ? it.section : null);
    const byCol = (l: WorkspaceItem[]) => l.map((it, i) => ({ it, i })).sort((a, b) => (a.it.col ?? 0) - (b.it.col ?? 0) || a.i - b.i).map((x) => x.it);
    return [...sections.flatMap((s) => byCol(list.filter((it) => of(it) === s.id))), ...byCol(list.filter((it) => of(it) == null))];
  }
  function emit(list: WorkspaceItem[]) {
    // Les succès rattachés prennent la section de leur parent (rangement, et place s'ils sont détachés)
    const n = nesting(list);
    const sec = new Map(list.filter((it) => it.card_id == null).map((it) => [it.achievement_id, it.section ?? null]));
    const synced = list.map((it) => {
      const p = it.card_id == null && it.achievement_id != null ? n.get(it.achievement_id) : undefined;
      return p != null ? { ...it, section: sec.get(p) ?? null } : it;
    });
    onchange(ordered(synced));
  }
  function update(key: CardKey, patch: Partial<WorkspaceItem>) {
    emit(items.map((it) => (cardKey(it) === key ? { ...it, ...patch } : it)));
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
  /** Section dont on choisit la couleur (nuancier affiché sous son en-tête). */
  let coloring = $state<number | null>(null);

  // Réordonner les sections : glisser natif par la poignée ⠿ de l'en-tête
  const SECTION_MIME = "application/x-grimoire-section";
  let sectionDrag = $state<number | null>(null);
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

  // ----- Déplacement par clics -----
  /** Carte en cours de déplacement. */
  let moving = $state<CardKey | null>(null);
  /** Carte qui vient d'être déplacée : brièvement soulignée, et amenée à l'écran. */
  let landed = $state<CardKey | null>(null);
  const canMove = $derived(filters.sort === "personal");
  const movingLabel = $derived.by(() => {
    if (moving == null) return "";
    if (moving < 0) return freeById.get(-moving)?.title || "Carte sans titre";
    const a = byId.get(moving);
    return a ? (spoiler(a) ? "succès caché" : a.name) : "";
  });
  // Plus de tri « Mon ordre » : le déplacement s'arrête
  $effect(() => {
    if (!canMove) moving = null;
  });

  async function moved(key: CardKey) {
    moving = null;
    landed = key;
    setTimeout(() => {
      if (landed === key) landed = null;
    }, 1400);
    await tick();
    // Colonnes recalculées puis mesurées : on attend deux images avant d'amener la carte à l'écran
    requestAnimationFrame(() =>
      requestAnimationFrame(() =>
        document.querySelector(`[data-card="${key}"]`)?.scrollIntoView({ block: "nearest", behavior: "smooth" }),
      ),
    );
  }

  /** Place la carte déplacée au-dessus (`after` faux) ou en dessous de `target`, dans sa section et sa colonne. */
  function placeNear(target: WorkspaceItem, after: boolean) {
    const key = moving;
    if (key == null || key === cardKey(target)) return void (moving = null);
    const it = items.find((x) => cardKey(x) === key);
    if (!it) return;
    const rest = items.filter((x) => cardKey(x) !== key);
    const at = rest.indexOf(target) + (after ? 1 : 0);
    rest.splice(at, 0, { ...it, section: sectionOf(target), col: target.col ?? 0 });
    emit(rest);
    moved(key);
  }

  /** Met la carte déplacée en bas d'une colonne d'une section (null : hors section). */
  function toColumn(section: number | null, col: number) {
    const key = moving;
    const it = key != null ? items.find((x) => cardKey(x) === key) : undefined;
    if (key == null || !it) return;
    emit([...items.filter((x) => cardKey(x) !== key), { ...it, section, col }]);
    moved(key);
  }

  /** Met la carte à la fin d'une section (null : hors section), dans la colonne qui a le moins de cartes. */
  function toEnd(key: CardKey, section: number | null) {
    const it = items.find((x) => cardKey(x) === key);
    if (!it) return;
    emit([...items.filter((x) => cardKey(x) !== key), { ...it, section, col: null }]);
    moved(key);
  }

  /** Entrées du menu d'une carte : rattacher (succès), « Déplacer vers… » (à la fin de la section choisie). */
  function movesFor(it: WorkspaceItem) {
    const out: MenuItem[] = [];
    if (it.achievement_id != null && it.card_id == null) {
      const id = it.achievement_id;
      out.push({
        label: "Rattacher à un autre succès…",
        hint: "Obtenu en passant par un autre : il s'affichera dans sa carte",
        onclick: () => {
          moving = null;
          attaching = id;
        },
      });
    }
    if (!hasSections) return out;
    const here = sectionOf(it);
    out.push(...sections
      .filter((s) => s.id !== here)
      .map((s) => ({
        group: "Déplacer vers…",
        label: s.title || "Section sans titre",
        color: sectionColor(s.color),
        onclick: () => toEnd(cardKey(it), s.id),
      })));
    if (here != null)
      out.push({ group: "Déplacer vers…", label: "Autres succès (hors section)", color: "var(--color-muted)", onclick: () => toEnd(cardKey(it), null) });
    return out;
  }

  // ----- Rattachement par clics -----
  /** Succès en cours de rattachement : on clique ensuite sur le succès qui le contiendra. */
  let attaching = $state<number | null>(null);
  const attachingLabel = $derived.by(() => {
    const a = attaching != null ? byId.get(attaching) : undefined;
    return a ? (spoiler(a) ? "succès caché" : a.name) : "";
  });

  /** Rattache le succès choisi (et ceux qu'il contenait déjà : un seul niveau) au succès `target`. */
  function attachTo(target: WorkspaceItem) {
    const id = attaching;
    attaching = null;
    const tid = target.achievement_id;
    if (id == null || tid == null || target.card_id != null || tid === id) return;
    const goes = (x: WorkspaceItem) => x.card_id == null && (x.achievement_id === id || parentOf(x) === id);
    const going = items.filter(goes).map((x) => ({ ...x, parent: tid }));
    const rest = items.filter((x) => !goes(x));
    // Après le parent et ses succès déjà rattachés
    let at = rest.indexOf(target);
    rest.forEach((x, i) => {
      if (parentOf(x) === tid) at = Math.max(at, i);
    });
    rest.splice(at + 1, 0, ...going);
    emit(rest);
    moved(cardKey(target));
  }

  /** Rend sa propre carte à un succès rattaché, juste après son parent. */
  function detach(id: number) {
    const it = itemOfAch.get(id);
    const p = nest.get(id);
    if (!it || p == null) return;
    const rest = items.filter((x) => x !== it);
    const at = rest.findIndex((x) => x.card_id == null && x.achievement_id === p);
    rest.splice(at + 1, 0, { ...it, parent: null, section: sectionOf(it), col: itemOfAch.get(p)?.col ?? 0 });
    emit(rest);
    moved(id);
  }

  /** Carte (clé) qui vient d'être cochée ou décochée : sa coche s'anime. */
  let justToggled = $state<CardKey | null>(null);
  const spoiler = (a: AchievementRow) => settings.antiSpoiler && a.hidden && !a.unlocked && !revealed.has(a.id);
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === "Escape" && moving != null) moving = null;
    if (e.key === "Escape" && attaching != null) attaching = null;
  }}
/>

{#snippet card(it: WorkspaceItem)}
  {@const key = cardKey(it)}
  {@const a = it.achievement_id != null ? byId.get(it.achievement_id) : undefined}
  {@const free = it.card_id != null ? freeById.get(it.card_id) : undefined}
  {@const kids = a ? kidRows(a.id) : []}
  <div
    class="relative {landed === key ? 'landed' : ''} {attaching != null && (free || key === attaching) ? 'opacity-50' : ''}"
    data-card={key}
  >
    {#if free}
      <WorkspaceCard
        {free}
        focusTitle={focusKey === key}
        pinned={it.pinned}
        spoiler={false}
        animate={justToggled === key}
        {imagesDir}
        {canMove}
        moving={moving === key}
        moves={movesFor(it)}
        ontoggle={() => {
          justToggled = key;
          ontogglefree(free.id);
        }}
        onreveal={() => {}}
        onremove={() => ondeletefree(free.id)}
        onpin={() => update(key, { pinned: !it.pinned })}
        onmove={() => {
          attaching = null;
          moving = moving === key ? null : key;
        }}
        onrename={(title) => onrename(free.id, title)}
      />
    {:else if a}
      <WorkspaceCard
        {a}
        pinned={it.pinned}
        spoiler={spoiler(a)}
        animate={justToggled === a.id}
        {imagesDir}
        {canMove}
        moving={moving === key}
        moves={movesFor(it)}
        ontoggle={() => {
          justToggled = a.id;
          ontoggle(a);
        }}
        onreveal={() => revealed.add(a.id)}
        onremove={() => {}}
        onpin={() => update(key, { pinned: !it.pinned })}
        onmove={() => {
          attaching = null;
          moving = moving === key ? null : key;
        }}
        nestedCount={kids.length}
      >
        {#snippet nested()}
          {#each kids as { it: k, a: ka } (ka.id)}
            <NestedAchievement
              a={ka}
              spoiler={spoiler(ka)}
              animate={justToggled === ka.id}
              folded={k.collapsed}
              {imagesDir}
              ontoggle={() => {
                justToggled = ka.id;
                ontoggle(ka);
              }}
              onreveal={() => revealed.add(ka.id)}
              ondetach={() => detach(ka.id)}
              onfold={() => update(ka.id, { collapsed: !k.collapsed })}
            />
          {/each}
        {/snippet}
      </WorkspaceCard>
    {/if}
    {#if attaching != null && a && a.id !== attaching}
      <!-- Cible du rattachement : toute la carte d'un autre succès -->
      <button
        onclick={() => attachTo(it)}
        aria-label="Rattacher dans ce succès"
        class="move-target absolute inset-0 z-10 flex items-start justify-center rounded-[4px] border-2 border-dashed border-transparent
          transition hover:border-accent hover:bg-accent/10"
      >
        <span class="mt-1 rounded bg-accent px-2 py-0.5 text-xs font-semibold text-on-accent shadow">↳ Rattacher dans ce succès</span>
      </button>
    {/if}
    {#if moving != null && moving !== key}
      <!-- Cibles du déplacement : moitié haute (au-dessus), moitié basse (en dessous) -->
      <div class="absolute inset-0 z-10 flex flex-col">
        <button onclick={() => placeNear(it, false)} aria-label="Placer au-dessus de cette carte" class="half half-top">
          <span>↑ Placer au-dessus</span>
        </button>
        <button onclick={() => placeNear(it, true)} aria-label="Placer en dessous de cette carte" class="half half-bottom">
          <span>↓ Placer en dessous</span>
        </button>
      </div>
    {/if}
  </div>
{/snippet}

{#snippet cards(list: WorkspaceItem[], g: Group)}
  {@const cols = columnsOf(g)}
  {@const colOfKey = colMap(list, cols)}
  {#if list.length || moving != null}
    <Columns items={list} key={cardKey} columns={cols} colOf={(it) => colOfKey.get(cardKey(it)) ?? 0} footer={moving != null ? foot : undefined}>
      {#snippet children(it)}{@render card(it)}{/snippet}
    </Columns>
    {#snippet foot(c: number)}
      <button
        onclick={() => toColumn(g.section?.id ?? null, c)}
        class="grid h-full w-full place-items-center rounded-[4px] border-2 border-dashed border-ok/50 text-xs font-semibold text-ok
          transition hover:border-ok hover:bg-ok/10"
      >
        Mettre ici
      </button>
    {/snippet}
  {:else if g.items.length === 0}
    <p class="px-1 py-2 text-sm text-muted">
      Section vide : choisissez « Déplacer vers… » dans le menu ⋯ d'une carte, ou ⇅ puis « Mettre ici ».
    </p>
  {:else}
    <p class="px-1 py-2 text-sm text-muted">Rien à afficher avec ces filtres.</p>
  {/if}
{/snippet}

{#snippet endButton(section: number | null)}
  {#if moving != null}
    <button
      onclick={() => toEnd(moving!, section)}
      class="ml-auto shrink-0 rounded-md bg-accent px-2 py-0.5 text-xs font-semibold text-on-accent hover:brightness-110"
    >
      Mettre à la fin {section == null ? "des autres succès" : "de cette section"}
    </button>
  {/if}
{/snippet}

<!-- Nombre de colonnes d'un groupe -->
{#snippet colsStepper(g: Group)}
  {@const cols = columnsOf(g)}
  <span class="flex shrink-0 items-center gap-0.5 text-xs text-muted" role="group" aria-label="Nombre de colonnes">
    <span class="mr-0.5" title="Nombre de colonnes">Colonnes</span>
    <button
      onclick={() => setColumns(g, cols - 1)}
      disabled={cols <= 1}
      aria-label="Une colonne de moins"
      title="Une colonne de moins (ses cartes vont en bas de la dernière colonne)"
      class="grid h-6 w-6 place-items-center rounded transition hover:bg-raised hover:text-fg disabled:opacity-30">−</button
    >
    <span class="w-3 text-center font-semibold text-fg tabular-nums">{cols}</span>
    <button
      onclick={() => setColumns(g, cols + 1)}
      disabled={cols >= MAX_COLUMNS}
      aria-label="Une colonne de plus"
      title="Une colonne de plus"
      class="grid h-6 w-6 place-items-center rounded transition hover:bg-raised hover:text-fg disabled:opacity-30">+</button
    >
  </span>
{/snippet}

<!-- En-tête d'une section : poignée, repli, couleur, titre, bilan, menu -->
{#snippet head(g: Group, s: Section, index: number)}
  {@const t = tally(g)}
  <div
    data-section-head
    role="group"
    aria-label={`Section « ${s.title || "sans titre"} »`}
    class="section-head flex items-center gap-1.5 px-1.5 py-1
      {s.collapsed && coloring !== s.id && deleting !== s.id ? 'rounded-b-[5px]' : 'border-b'}"
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
    {@render endButton(s.id)}
    {#if !s.collapsed}{@render colsStepper(g)}{/if}
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
        {
          label: "Supprimer la section…",
          onclick: () => (g.items.length === 0 ? ondeletesection(s.id) : (deleting = s.id)),
        },
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
    <div class="m-2 flex flex-wrap items-center gap-2 rounded border border-danger/40 bg-danger/10 p-3 text-sm">
      <p class="min-w-0 flex-1">
        Supprimer la section « {s.title || "sans titre"} » ? Ses {g.items.length} carte{g.items.length > 1 ? "s" : ""} passeront
        dans « Autres succès », avec leurs guides.
      </p>
      <button onclick={() => (deleting = null)} class="rounded-md px-2 py-0.5 hover:bg-raised">Annuler</button>
      <button
        onclick={() => {
          deleting = null;
          ondeletesection(s.id);
        }}
        class="rounded-md bg-danger px-2 py-0.5 font-medium text-white">Supprimer la section</button
      >
    </div>
  {/if}
{/snippet}

<div class="flex min-h-full flex-col {sectionDrag != null ? 'section-dragging' : ''}">
  <!-- Barre d'outils : recherche, filtres, tri, ajouts -->
  <div class="sticky top-0 z-20 border-b border-line bg-bg/95 px-6 py-2.5 backdrop-blur">
    <div class="flex flex-wrap items-center gap-2 text-sm">
      <div class="relative">
        <input
          bind:value={filters.text}
          type="search"
          placeholder="Rechercher un succès…"
          aria-label="Rechercher un succès"
          class="w-56 rounded-lg border border-line bg-bg py-1 pr-3 pl-7 outline-none focus:border-accent"
        />
        <span class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-muted" aria-hidden="true">⌕</span>
      </div>
      <div class="inline-flex rounded-lg border border-line bg-panel p-0.5" role="group" aria-label="Filtrer par état">
        {#each stateTabs as t (t.value)}
          <button
            onclick={() => setState(t.value)}
            aria-pressed={filters.state === t.value}
            class="rounded-md px-2.5 py-0.5 transition
              {filters.state === t.value ? 'bg-raised font-medium text-fg' : 'text-muted hover:text-fg'}"
          >
            {t.label} <span class="text-xs text-muted">{t.count()}</span>
          </button>
        {/each}
      </div>
      <select bind:value={filters.sort} aria-label="Trier" class="rounded-lg border border-line bg-panel px-2 py-1 outline-none focus:border-accent">
        {#each sortOptions as s (s.value)}<option value={s.value}>{s.label}</option>{/each}
      </select>
      <select bind:value={filters.rarity} aria-label="Filtrer par rareté" class="rounded-lg border border-line bg-panel px-2 py-1 outline-none focus:border-accent">
        {#each rarityTiers as t (t.value)}<option value={t.value}>{t.label}</option>{/each}
      </select>
      <select bind:value={filters.tag} aria-label="Filtrer par étiquette" class="rounded-lg border border-line bg-panel px-2 py-1 outline-none focus:border-accent">
        <option value="all">Toutes les étiquettes</option>
        <option value="none">Sans étiquette</option>
        {#each tags.list as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
      </select>
      <label class="flex items-center gap-1.5 text-muted" title="Seulement les succès qui ont au moins un bloc de guide">
        <input type="checkbox" bind:checked={guidedOnly} class="accent-accent" /> Avec guide
      </label>
      <button
        onclick={() => setAntiSpoiler(!settings.antiSpoiler)}
        aria-pressed={settings.antiSpoiler}
        title="Mode anti-spoiler : floute les succès cachés non obtenus"
        class="rounded-lg border px-2 py-0.5 text-xs transition
          {settings.antiSpoiler ? 'border-accent/60 bg-accent/15 text-fg' : 'border-line text-muted hover:text-fg'}"
      >
        Anti-spoiler {settings.antiSpoiler ? "activé" : "désactivé"}
      </button>
    </div>
    <div class="mt-1.5 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted">
      <span>{shownCount} affiché{shownCount > 1 ? "s" : ""}</span>
      {#if filtered}<button onclick={resetFilters} class="underline hover:text-fg">réinitialiser les filtres</button>{/if}
      <button
        onclick={onaddfree}
        class="rounded-md border border-line px-2 py-0.5 transition hover:bg-raised hover:text-fg"
        title="Carte rattachée au jeu, pas à un succès : route générale, objets à ne pas rater, compteur commun…"
      >
        + Carte libre
      </button>
      <button
        onclick={onaddsection}
        class="rounded-md border border-line px-2 py-0.5 transition hover:bg-raised hover:text-fg"
        title="Groupe nommé et repliable : un chapitre, les collectibles, le grind…"
      >
        + Section
      </button>
      {#if hasSections}
        <button onclick={() => setAllSections(true)} class="underline hover:text-fg">replier les sections</button>
        <button onclick={() => setAllSections(false)} class="underline hover:text-fg">déplier les sections</button>
      {/if}
      {#if !canMove}<span>Pour déplacer des cartes, choisissez le tri « Mon ordre ».</span>{/if}
    </div>
    {#if moving != null}
      <div class="mt-2 flex flex-wrap items-center gap-3 rounded-lg border border-accent/60 bg-raised px-3 py-2 text-sm shadow-lg">
        <span class="min-w-0 flex-1">
          Déplacement de <strong>« {movingLabel} »</strong> : cliquez en haut d'une carte pour la placer au-dessus, en
          bas pour la placer en dessous, ou sur « Mettre ici » au bas d'une colonne.
        </span>
        <button onclick={() => (moving = null)} class="rounded-md border border-line px-2 py-0.5 hover:bg-panel">Annuler (Échap)</button>
      </div>
    {/if}
    {#if attaching != null}
      <div class="mt-2 flex flex-wrap items-center gap-3 rounded-lg border border-accent/60 bg-raised px-3 py-2 text-sm shadow-lg">
        <span class="min-w-0 flex-1">
          Rattachement de <strong>« {attachingLabel} »</strong> : cliquez sur le succès dans lequel le ranger (celui qui
          vous le fera obtenir en passant). La recherche aide à le trouver.
        </span>
        <button onclick={() => (attaching = null)} class="rounded-md border border-line px-2 py-0.5 hover:bg-panel">Annuler (Échap)</button>
      </div>
    {/if}
  </div>

  <div class="flex flex-1 flex-col gap-3 px-6 pt-3 pb-6">
    {#each shown as { g, list }, i (g.key)}
      {#if g.section}
        {@const s = g.section}
        <section
          aria-label={s.title || "Section sans titre"}
          ondragover={(e) => sectionDragOver(e, i)}
          ondrop={sectionDrop}
          class="zone-frame relative {sectionDrag === s.id ? 'opacity-50' : ''}"
          style:--zone-color={sectionColor(s.color)}
        >
          {#if dropIndex === i && sectionDrag != null}<div class="drop-line -top-2"></div>{/if}
          {@render head(g, s, i)}
          {#if !s.collapsed}<div class="p-2.5">{@render cards(list, g)}</div>{/if}
          {#if dropIndex === i + 1 && i === sections.length - 1 && sectionDrag != null}<div class="drop-line -bottom-2"></div>{/if}
        </section>
      {:else if hasSections}
        {@const t = tally(g)}
        <section aria-label="Autres succès" class="zone-frame" style:--zone-color="var(--color-muted)">
          <div class="section-head flex items-center gap-2 border-b px-3 py-1.5 text-sm">
            <span class="font-semibold">Autres succès</span>
            {@render endButton(null)}
            <span class="ml-auto"></span>
            {@render colsStepper(g)}
            <span class="text-xs text-muted tabular-nums">{t.done} / {t.total}</span>
          </div>
          <div class="p-2.5">{@render cards(list, g)}</div>
        </section>
      {:else}
        <div class="mb-1 flex justify-end">{@render colsStepper(g)}</div>
        {@render cards(list, g)}
      {/if}
    {/each}
  </div>
</div>

<style>
  /* Pendant qu'une section est glissée, les cartes (et leurs vidéos) ne captent pas le glisser */
  .section-dragging :global([data-card]) {
    pointer-events: none;
  }
  /* Cadre d'une section dans sa couleur */
  .zone-frame {
    border: 1px solid color-mix(in srgb, var(--zone-color) 70%, transparent);
    background: color-mix(in srgb, var(--zone-color) 7%, var(--color-bg));
    border-radius: 6px;
  }
  .section-head,
  .section-band {
    background: color-mix(in srgb, var(--zone-color) 24%, var(--color-panel));
    border-color: color-mix(in srgb, var(--zone-color) 55%, transparent);
  }
  .section-head {
    border-radius: 5px 5px 0 0;
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
  /* Cible de déplacement : le libellé n'apparaît qu'au survol */
  .move-target:not(:hover):not(:focus-visible) span {
    visibility: hidden;
  }
  /* Moitiés d'une carte pendant un déplacement : trait vert au-dessus ou en dessous au survol */
  .half {
    position: relative;
    flex: 1;
    display: flex;
    justify-content: center;
    border-radius: 4px;
  }
  .half-top {
    align-items: flex-start;
  }
  .half-bottom {
    align-items: flex-end;
  }
  .half span {
    visibility: hidden;
    margin: 2px 0;
    border-radius: 4px;
    background: var(--color-ok);
    color: var(--color-bg);
    padding: 0 8px;
    font-size: 11px;
    font-weight: 600;
    line-height: 18px;
    white-space: nowrap;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.3);
  }
  .half:hover,
  .half:focus-visible {
    background: color-mix(in srgb, var(--color-ok) 10%, transparent);
    outline: none;
  }
  .half:hover span,
  .half:focus-visible span {
    visibility: visible;
  }
  .half::after {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    height: 3px;
    border-radius: 2px;
    background: var(--color-ok);
    opacity: 0;
  }
  .half-top::after {
    top: -6px;
  }
  .half-bottom::after {
    bottom: -6px;
  }
  .half:hover::after,
  .half:focus-visible::after {
    opacity: 1;
  }
  /* Carte qui vient d'être déplacée */
  .landed :global(.ws-card) {
    animation: landed 1.3s ease-out;
  }
  @keyframes landed {
    0% {
      box-shadow: 0 0 0 3px var(--color-accent);
    }
    100% {
      box-shadow: 0 0 0 0 transparent;
    }
  }
  :global([data-reduced-motion]) .landed :global(.ws-card) {
    animation: none;
  }
</style>
