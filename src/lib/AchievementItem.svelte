<script lang="ts">
  import { rarityTier } from "./filters";
  import type { AchievementRow } from "./library";

  let {
    a,
    spoiler,
    animate,
    ontoggle,
    onreveal,
  }: {
    a: AchievementRow;
    /** Nom et description à flouter (succès caché, non obtenu, anti-spoiler actif). */
    spoiler: boolean;
    /** Joue l'animation de coche (seulement juste après un clic). */
    animate: boolean;
    ontoggle: (a: AchievementRow) => void;
    onreveal: (a: AchievementRow) => void;
  } = $props();

  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | undefined;

  async function copyId() {
    if (!a.api_name) return;
    try {
      await navigator.clipboard.writeText(a.api_name);
      copied = true;
      clearTimeout(copyTimer);
      copyTimer = setTimeout(() => (copied = false), 1500);
    } catch {
      copied = false;
    }
  }

  const icon = $derived(a.unlocked ? a.icon_url : (a.icon_gray_url ?? a.icon_url));
  const tier = $derived(rarityTier(a.rarity));
  const rarityColor = $derived(tier === "ultra" ? "text-gold" : tier === "rare" ? "text-rare" : "text-muted");
  const barColor = $derived(tier === "ultra" ? "bg-gold" : tier === "rare" ? "bg-rare" : "bg-muted/60");

  const fmtDate = (t: number) =>
    new Date(t * 1000).toLocaleDateString("fr-FR", { day: "numeric", month: "short", year: "numeric" });
</script>

<li class="group flex items-center gap-3 px-3 py-2.5 transition-colors hover:bg-raised/50 {a.removed ? 'opacity-50' : ''}">
  <!-- Coche -->
  <button
    onclick={() => ontoggle(a)}
    aria-pressed={a.unlocked}
    aria-label={a.unlocked ? `Marquer « ${spoiler ? "succès caché" : a.name} » comme non obtenu` : `Marquer « ${spoiler ? "succès caché" : a.name} » comme obtenu`}
    title={a.unlocked ? "Marquer comme non obtenu" : "Marquer comme obtenu"}
    class="grid h-6 w-6 shrink-0 place-items-center rounded-full border-2 transition
      focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent
      {a.unlocked ? 'border-ok bg-ok text-bg' : 'border-line hover:border-muted'}"
  >
    {#if a.unlocked}
      <svg viewBox="0 0 16 16" class="h-3.5 w-3.5" aria-hidden="true">
        <path d="M3 8.5l3.2 3L13 4.5" fill="none" stroke="currentColor" stroke-width="2.4"
          stroke-linecap="round" stroke-linejoin="round" class={animate ? "draw" : ""} />
      </svg>
    {/if}
  </button>

  <!-- Icône -->
  <div
    class="h-12 w-12 shrink-0 overflow-hidden rounded-md bg-raised
      {a.unlocked ? 'shadow-[0_0_14px_-3px] shadow-accent/60' : ''}
      {animate && a.unlocked ? 'light-up' : ''}"
  >
    {#if icon}
      <img src={icon} alt="" loading="lazy" class="h-full w-full {a.unlocked ? '' : 'opacity-70 grayscale'}" />
    {/if}
  </div>

  <!-- Texte -->
  <div class="min-w-0 flex-1">
    {#if spoiler}
      <button
        onclick={() => onreveal(a)}
        class="block w-full rounded text-left focus-visible:outline-2 focus-visible:outline-accent"
        title="Succès caché : cliquer pour révéler"
      >
        <span class="flex items-center gap-2">
          <span class="truncate font-medium blur-[6px] select-none" aria-hidden="true">{a.name}</span>
          <span class="shrink-0 rounded bg-raised px-1.5 py-0.5 text-[10px] text-muted">caché · cliquer pour révéler</span>
        </span>
        <span class="sr-only">Succès caché, activer pour révéler</span>
        <span class="block truncate text-sm text-muted blur-[6px] select-none" aria-hidden="true">{a.description || "—"}</span>
      </button>
    {:else}
      <div class="flex items-center gap-2">
        <span class="truncate font-medium {a.unlocked ? '' : 'text-fg/85'}">{a.name}</span>
        {#if a.hidden}<span class="shrink-0 rounded bg-raised px-1.5 py-0.5 text-[10px] text-muted">caché</span>{/if}
        {#if a.dlc_appid}<span class="shrink-0 rounded bg-raised px-1.5 py-0.5 text-[10px] text-muted">DLC</span>{/if}
        {#if a.removed}<span class="shrink-0 rounded bg-raised px-1.5 py-0.5 text-[10px] text-muted">retiré de Steam</span>{/if}
      </div>
      <p class="text-sm text-muted">{a.description || "—"}</p>
    {/if}
    {#if a.api_name}
      <button
        onclick={copyId}
        class="mt-0.5 inline-flex max-w-full items-center gap-1 rounded px-1 font-mono text-[11px] text-muted/70
          transition hover:bg-raised hover:text-fg focus-visible:outline-2 focus-visible:outline-accent"
        title="Copier l'ID technique"
      >
        <span class="truncate">{a.api_name}</span>
        <span class="shrink-0 {copied ? 'text-ok' : 'opacity-0 group-hover:opacity-100'}">{copied ? "✓ copié" : "⧉"}</span>
      </button>
    {/if}
  </div>

  <!-- Rareté et date -->
  <div class="w-28 shrink-0 text-right text-xs">
    {#if a.rarity != null}
      <div class="font-semibold {rarityColor}">{a.rarity.toFixed(1)} %</div>
      <div class="mt-1 ml-auto h-1 w-16 overflow-hidden rounded-full bg-line" aria-hidden="true">
        <div class="h-full rounded-full {barColor}" style="width: {Math.max(a.rarity, 2)}%"></div>
      </div>
    {/if}
    {#if a.unlocked}
      <div class="mt-1 text-ok">
        {#if a.unlock_time}{fmtDate(a.unlock_time)}{:else if a.manual}coché à la main{:else}obtenu{/if}
      </div>
    {/if}
  </div>
</li>

<style>
  /* La coche se dessine */
  .draw {
    stroke-dasharray: 16;
    stroke-dashoffset: 16;
    animation: draw 0.35s ease-out 0.05s forwards;
  }
  @keyframes draw {
    to {
      stroke-dashoffset: 0;
    }
  }

  /* L'icône s'allume */
  .light-up {
    animation: light-up 0.6s ease-out;
  }
  @keyframes light-up {
    0% {
      transform: scale(0.92);
      filter: grayscale(1) brightness(0.8);
    }
    50% {
      transform: scale(1.08);
      filter: grayscale(0) brightness(1.4);
    }
    100% {
      transform: scale(1);
      filter: none;
    }
  }
</style>
