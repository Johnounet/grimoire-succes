<script lang="ts">
  // Liste complète des succès d'un jeu (panneau de droite) : recherche, filtres, tris,
  // sélection et glisser vers l'espace de travail.
  import { SvelteSet } from "svelte/reactivity";
  import AchievementItem from "./AchievementItem.svelte";
  import {
    apply,
    counts,
    defaultFilters,
    isDefault,
    rarityTiers,
    sortOptions,
    type Filters,
    type StateFilter,
  } from "./filters";
  import { DRAG_MIME, type AchievementRow } from "./library";
  import { setAntiSpoiler, settings, ui } from "./settings.svelte";
  import { tags } from "./tags.svelte";

  let {
    achievements,
    personalOrder,
    revealed,
    ontoggle,
    onadd,
  }: {
    achievements: AchievementRow[];
    /** Rang de chaque succès dans l'espace de travail (présence = déjà dans l'espace). */
    personalOrder: Map<number, number>;
    /** Succès cachés révélés (partagé avec les cartes). */
    revealed: SvelteSet<number>;
    ontoggle: (a: AchievementRow) => void;
    onadd: (ids: number[]) => void;
  } = $props();

  let filters = $state<Filters>(defaultFilters());
  const selected = new SvelteSet<number>();
  let lastClicked: number | null = null;
  /** Succès qui vient d'être coché : seul lui joue l'animation. */
  let justToggled = $state<number | null>(null);

  const shown = $derived(apply(achievements, filters, personalOrder, tags.byAch));
  // Étiquette filtrée supprimée entre-temps : on revient à « toutes »
  $effect(() => {
    const t = filters.tag;
    if (typeof t === "number" && !tags.list.some((x) => x.id === t)) filters.tag = "all";
  });
  const n = $derived(counts(achievements));
  const selectable = (a: AchievementRow) => !a.removed && !personalOrder.has(a.id);

  // Un succès ajouté à l'espace sort de la sélection
  $effect(() => {
    for (const id of selected) if (personalOrder.has(id)) selected.delete(id);
  });

  const stateTabs: { value: StateFilter; label: string; count: () => number }[] = [
    { value: "all", label: "Tous", count: () => n.all },
    { value: "unlocked", label: "Obtenus", count: () => n.unlocked },
    { value: "locked", label: "À obtenir", count: () => n.locked },
  ];

  function toggle(a: AchievementRow) {
    justToggled = a.id;
    ontoggle(a);
  }

  /** Clic sur une case ; Maj+clic sélectionne toute la plage depuis le dernier clic. */
  function select(a: AchievementRow, range: boolean) {
    const willSelect = !selected.has(a.id);
    const i = shown.findIndex((x) => x.id === a.id);
    const j = range && lastClicked != null ? shown.findIndex((x) => x.id === lastClicked) : -1;
    const targets = j >= 0 ? shown.slice(Math.min(i, j), Math.max(i, j) + 1) : [a];
    for (const t of targets.filter(selectable)) {
      if (willSelect) selected.add(t.id);
      else selected.delete(t.id);
    }
    lastClicked = a.id;
  }

  function selectAllShown() {
    for (const a of shown.filter(selectable)) selected.add(a.id);
  }

  function addSelected() {
    // Dans l'ordre affiché
    onadd(shown.filter((a) => selected.has(a.id)).map((a) => a.id));
    selected.clear();
  }

  /** Glisser natif : la sélection entière si on attrape un succès sélectionné, sinon ce seul succès. */
  function dragStart(a: AchievementRow, e: DragEvent) {
    if (!e.dataTransfer) return;
    const ids = selected.has(a.id) ? shown.filter((x) => selected.has(x.id)).map((x) => x.id) : [a.id];
    e.dataTransfer.setData(DRAG_MIME, JSON.stringify(ids));
    e.dataTransfer.effectAllowed = "copy";
    ui.draggingFromPanel = true;
    ui.draggingIds = ids;
  }

  const isSpoiler = (a: AchievementRow) => settings.antiSpoiler && a.hidden && !a.unlocked && !revealed.has(a.id);
</script>

<div class="flex min-h-full flex-col">
  <!-- Barre d'outils -->
  <div class="sticky top-0 z-10 space-y-2 border-b border-line bg-panel/95 px-3 py-3 backdrop-blur">
    <div class="relative">
      <input
        bind:value={filters.text}
        type="search"
        placeholder="Rechercher un succès, une description, un ID…"
        aria-label="Rechercher un succès"
        class="w-full rounded-lg border border-line bg-bg py-1.5 pr-3 pl-8 text-sm outline-none focus:border-accent"
      />
      <span class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-muted" aria-hidden="true">⌕</span>
    </div>

    <div class="flex flex-wrap gap-2">
      <select
        bind:value={filters.rarity}
        aria-label="Filtrer par rareté"
        class="min-w-0 flex-1 rounded-lg border border-line bg-bg px-2 py-1.5 text-sm outline-none focus:border-accent"
      >
        {#each rarityTiers as t (t.value)}<option value={t.value}>{t.label}</option>{/each}
      </select>
      <select
        bind:value={filters.sort}
        aria-label="Trier"
        class="min-w-0 flex-1 rounded-lg border border-line bg-bg px-2 py-1.5 text-sm outline-none focus:border-accent"
      >
        {#each sortOptions as s (s.value)}<option value={s.value}>{s.label}</option>{/each}
      </select>
      <select
        bind:value={filters.tag}
        aria-label="Filtrer par étiquette"
        class="min-w-0 basis-full rounded-lg border border-line bg-bg px-2 py-1.5 text-sm outline-none focus:border-accent"
      >
        <option value="all">Toutes les étiquettes</option>
        <option value="none">Sans étiquette</option>
        {#each tags.list as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
      </select>
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <div class="inline-flex rounded-lg border border-line bg-bg p-0.5" role="group" aria-label="Filtrer par état">
        {#each stateTabs as t (t.value)}
          <button
            onclick={() => (filters.state = t.value)}
            aria-pressed={filters.state === t.value}
            class="rounded-md px-2.5 py-1 text-sm transition
              {filters.state === t.value ? 'bg-raised font-medium text-fg' : 'text-muted hover:text-fg'}"
          >
            {t.label} <span class="text-xs text-muted">{t.count()}</span>
          </button>
        {/each}
      </div>
      <button
        onclick={() => setAntiSpoiler(!settings.antiSpoiler)}
        aria-pressed={settings.antiSpoiler}
        title="Mode anti-spoiler : floute les succès cachés non obtenus"
        class="ml-auto rounded-lg border px-2 py-1 text-xs transition
          {settings.antiSpoiler ? 'border-accent/60 bg-accent/15 text-fg' : 'border-line text-muted hover:text-fg'}"
      >
        Anti-spoiler {settings.antiSpoiler ? "activé" : "désactivé"}
      </button>
    </div>

    <p class="text-xs text-muted">
      {shown.length} affiché{shown.length > 1 ? "s" : ""}
      {#if !isDefault(filters)}
        · <button onclick={() => (filters = { ...defaultFilters(), sort: filters.sort })} class="underline hover:text-fg">
          réinitialiser les filtres
        </button>
      {/if}
      · <button onclick={selectAllShown} class="underline hover:text-fg">tout sélectionner</button>
    </p>
  </div>

  <!-- Liste -->
  <div class="flex-1">
    {#if shown.length === 0}
      <p class="p-8 text-center text-sm text-muted">Aucun succès ne correspond à ces filtres.</p>
    {:else}
      <ul class="divide-y divide-line">
        {#each shown as a (a.id)}
          <AchievementItem
            {a}
            spoiler={isSpoiler(a)}
            animate={justToggled === a.id}
            selected={selected.has(a.id)}
            inWorkspace={personalOrder.has(a.id)}
            ontoggle={toggle}
            onreveal={(x) => revealed.add(x.id)}
            onselect={select}
            ondragstart={dragStart}
            ondragend={() => (ui.draggingFromPanel = false)}
          />
        {/each}
      </ul>
    {/if}
  </div>

  <!-- Action sur la sélection -->
  {#if selected.size > 0}
    <div class="sticky bottom-0 flex items-center gap-2 border-t border-line bg-raised px-3 py-2.5 shadow-[0_-8px_20px_-12px] shadow-black">
      <span class="flex-1 text-sm">{selected.size} sélectionné{selected.size > 1 ? "s" : ""}</span>
      <button onclick={() => selected.clear()} class="rounded-md px-2.5 py-1 text-sm text-muted hover:text-fg">Annuler</button>
      <button onclick={addSelected} class="rounded-md bg-accent px-3 py-1 text-sm font-medium text-on-accent hover:brightness-110">
        Ajouter à l'espace
      </button>
    </div>
  {/if}
</div>
