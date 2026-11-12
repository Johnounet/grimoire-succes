<script lang="ts">
  // Carte d'un succès dans l'espace de travail : repliable, déplaçable par sa poignée.
  import { dragHandle } from "svelte-dnd-action";
  import CopyId from "./CopyId.svelte";
  import { rarityText } from "./filters";
  import { columns, type AchievementRow, type Column } from "./library";

  let {
    a,
    column,
    collapsed,
    spoiler,
    animate,
    compact,
    ontoggle,
    onreveal,
    oncollapse,
    onremove,
    oncolumn,
  }: {
    a: AchievementRow;
    column: Column;
    collapsed: boolean;
    spoiler: boolean;
    animate: boolean;
    /** Vue colonnes : pas de sélecteur de colonne (on déplace la carte). */
    compact: boolean;
    ontoggle: () => void;
    onreveal: () => void;
    oncollapse: () => void;
    onremove: () => void;
    oncolumn: (c: Column) => void;
  } = $props();

  const icon = $derived(a.unlocked ? a.icon_url : (a.icon_gray_url ?? a.icon_url));
  const label = $derived(spoiler ? "succès caché" : a.name);
</script>

<article
  class="rounded-xl border bg-panel transition-colors
    {a.unlocked ? 'border-ok/30' : 'border-line'}"
  aria-label={label}
>
  <header class="flex items-center gap-2 p-2.5">
    <span
      use:dragHandle
      aria-label={`Déplacer la carte « ${label} »`}
      title="Glisser pour déplacer (ou Entrée puis flèches)"
      class="grid h-8 w-5 shrink-0 cursor-grab place-items-center rounded text-muted hover:bg-raised hover:text-fg active:cursor-grabbing"
      >⠿</span
    >

    <button
      onclick={ontoggle}
      aria-pressed={a.unlocked}
      aria-label={a.unlocked ? `Marquer « ${label} » comme non obtenu` : `Marquer « ${label} » comme obtenu`}
      title={a.unlocked ? "Marquer comme non obtenu" : "Marquer comme obtenu"}
      class="grid h-6 w-6 shrink-0 place-items-center rounded-full border-2 transition
        focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent
        {a.unlocked ? 'border-ok bg-ok text-bg' : 'border-line hover:border-muted'}"
    >
      {#if a.unlocked}
        <svg viewBox="0 0 16 16" class="h-3.5 w-3.5" aria-hidden="true">
          <path d="M3 8.5l3.2 3L13 4.5" fill="none" stroke="currentColor" stroke-width="2.4"
            stroke-linecap="round" stroke-linejoin="round" class={animate ? "check-draw" : ""} />
        </svg>
      {/if}
    </button>

    <div
      class="h-10 w-10 shrink-0 overflow-hidden rounded-md bg-raised
        {a.unlocked ? 'shadow-[0_0_14px_-3px] shadow-accent/60' : ''}
        {animate && a.unlocked ? 'light-up' : ''}"
    >
      {#if icon}
        <img src={icon} alt="" draggable="false" class="h-full w-full {a.unlocked ? '' : 'opacity-70 grayscale'}" />
      {/if}
    </div>

    <div class="min-w-0 flex-1">
      {#if spoiler}
        <button onclick={onreveal} class="block max-w-full text-left" title="Succès caché : cliquer pour révéler">
          <span class="block truncate font-medium blur-[6px] select-none" aria-hidden="true">{a.name}</span>
          <span class="sr-only">Succès caché, activer pour révéler</span>
        </button>
      {:else}
        <div class="font-medium {compact ? 'line-clamp-2 leading-snug break-words' : 'truncate'} {a.unlocked ? '' : 'text-fg/90'}">
          {a.name}
        </div>
      {/if}
      {#if a.rarity != null}
        <div class="text-xs font-semibold {rarityText(a.rarity)}">{a.rarity.toFixed(1)} %</div>
      {/if}
    </div>

    <button
      onclick={oncollapse}
      aria-expanded={!collapsed}
      aria-label={collapsed ? "Déplier la carte" : "Replier la carte"}
      title={collapsed ? "Déplier" : "Replier"}
      class="grid h-7 w-7 shrink-0 place-items-center rounded text-muted transition hover:bg-raised hover:text-fg"
    >
      <span class="transition-transform {collapsed ? '-rotate-90' : ''}">▾</span>
    </button>
    <button
      onclick={onremove}
      aria-label="Retirer de l'espace de travail"
      title="Retirer de l'espace de travail"
      class="grid h-7 w-7 shrink-0 place-items-center rounded text-muted transition hover:bg-raised hover:text-danger"
    >
      ✕
    </button>
  </header>

  {#if !collapsed}
    <div class="space-y-2 border-t border-line px-3 pt-2 pb-3">
      {#if spoiler}
        <button onclick={onreveal} class="block text-left text-sm text-muted">
          <span class="blur-[6px] select-none" aria-hidden="true">{a.description || "—"}</span>
          <span class="ml-1 rounded bg-raised px-1.5 py-0.5 text-[10px]">caché · cliquer pour révéler</span>
        </button>
      {:else}
        <p class="text-sm text-muted">{a.description || "—"}</p>
      {/if}

      <div class="flex flex-wrap items-center gap-2">
        {#if a.api_name}<CopyId id={a.api_name} />{/if}
        {#if !compact}
          <select
            value={column}
            onchange={(e) => oncolumn((e.currentTarget as HTMLSelectElement).value as Column)}
            aria-label="Colonne"
            class="ml-auto rounded-md border border-line bg-bg px-1.5 py-0.5 text-xs outline-none focus:border-accent"
          >
            {#each columns as c (c.id)}<option value={c.id}>{c.label}</option>{/each}
          </select>
        {/if}
      </div>

      <!-- Les blocs de guide (vidéo, image, texte, liste, compteur, chrono) arrivent à l'étape 6 -->
      <div class="rounded-lg border border-dashed border-line px-3 py-2 text-xs text-muted">
        Bientôt ici : vos vidéos, images, notes, listes, compteurs et chronos.
      </div>
    </div>
  {/if}
</article>
