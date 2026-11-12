<script lang="ts">
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
  import type { AchievementRow } from "./library";
  import { setAntiSpoiler, settings } from "./settings.svelte";

  let {
    achievements,
    ontoggle,
  }: {
    achievements: AchievementRow[];
    ontoggle: (a: AchievementRow) => void;
  } = $props();

  let filters = $state<Filters>(defaultFilters());
  const revealed = new SvelteSet<number>();
  /** Succès qui vient d'être coché : seul lui joue l'animation. */
  let justToggled = $state<number | null>(null);

  const shown = $derived(apply(achievements, filters));
  const n = $derived(counts(achievements));

  const stateTabs: { value: StateFilter; label: string; count: () => number }[] = [
    { value: "all", label: "Tous", count: () => n.all },
    { value: "unlocked", label: "Obtenus", count: () => n.unlocked },
    { value: "locked", label: "À obtenir", count: () => n.locked },
  ];

  function toggle(a: AchievementRow) {
    justToggled = a.id;
    ontoggle(a);
  }

  const isSpoiler = (a: AchievementRow) => settings.antiSpoiler && a.hidden && !a.unlocked && !revealed.has(a.id);
</script>

<div class="space-y-3">
  <!-- Barre d'outils -->
  <div class="sticky top-0 z-10 -mx-6 space-y-2 border-b border-line bg-bg/90 px-6 py-3 backdrop-blur">
    <div class="flex flex-wrap items-center gap-2">
      <div class="relative min-w-48 flex-1">
        <input
          bind:value={filters.text}
          type="search"
          placeholder="Rechercher un succès, une description, un ID…"
          aria-label="Rechercher un succès"
          class="w-full rounded-lg border border-line bg-panel py-1.5 pr-3 pl-8 text-sm outline-none focus:border-accent"
        />
        <span class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-muted" aria-hidden="true">⌕</span>
      </div>

      <select
        bind:value={filters.rarity}
        aria-label="Filtrer par rareté"
        class="rounded-lg border border-line bg-panel px-2 py-1.5 text-sm outline-none focus:border-accent"
      >
        {#each rarityTiers as t (t.value)}<option value={t.value}>{t.label}</option>{/each}
      </select>

      <select
        bind:value={filters.sort}
        aria-label="Trier"
        class="rounded-lg border border-line bg-panel px-2 py-1.5 text-sm outline-none focus:border-accent"
      >
        {#each sortOptions as s (s.value)}<option value={s.value}>{s.label}</option>{/each}
      </select>

      <button
        onclick={() => setAntiSpoiler(!settings.antiSpoiler)}
        aria-pressed={settings.antiSpoiler}
        title="Mode anti-spoiler : floute les succès cachés non obtenus"
        class="rounded-lg border px-2.5 py-1.5 text-sm transition
          {settings.antiSpoiler ? 'border-accent/60 bg-accent/15 text-fg' : 'border-line bg-panel text-muted hover:text-fg'}"
      >
        Anti-spoiler {settings.antiSpoiler ? "activé" : "désactivé"}
      </button>
    </div>

    <div class="flex flex-wrap items-center gap-3">
      <div class="inline-flex rounded-lg border border-line bg-panel p-0.5" role="group" aria-label="Filtrer par état">
        {#each stateTabs as t (t.value)}
          <button
            onclick={() => (filters.state = t.value)}
            aria-pressed={filters.state === t.value}
            class="rounded-md px-3 py-1 text-sm transition
              {filters.state === t.value ? 'bg-raised font-medium text-fg' : 'text-muted hover:text-fg'}"
          >
            {t.label} <span class="text-xs text-muted">{t.count()}</span>
          </button>
        {/each}
      </div>
      <span class="text-xs text-muted">
        {shown.length} affiché{shown.length > 1 ? "s" : ""}
        {#if !isDefault(filters)}
          · <button onclick={() => (filters = { ...defaultFilters(), sort: filters.sort })} class="underline hover:text-fg">
            réinitialiser les filtres
          </button>
        {/if}
      </span>
    </div>
  </div>

  <!-- Liste -->
  {#if shown.length === 0}
    <p class="rounded-xl border border-line bg-panel p-8 text-center text-sm text-muted">
      Aucun succès ne correspond à ces filtres.
    </p>
  {:else}
    <ul class="divide-y divide-line overflow-hidden rounded-xl border border-line bg-panel">
      {#each shown as a (a.id)}
        <AchievementItem
          {a}
          spoiler={isSpoiler(a)}
          animate={justToggled === a.id}
          ontoggle={toggle}
          onreveal={(x) => revealed.add(x.id)}
        />
      {/each}
    </ul>
  {/if}
</div>
