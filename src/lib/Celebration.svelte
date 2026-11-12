<script lang="ts">
  // Célébration du 100 % : pluie de confettis aux couleurs de la jaquette, puis le récap
  // (date du 100 %, temps de jeu Steam, succès le plus rare). Rejouable depuis la vitrine.
  import { backdrop, reduced } from "./motion.svelte";
  import { onMount } from "svelte";
  import { coverPalette, rain } from "./confetti";
  import { rarityText } from "./filters";
  import { api, formatDate, formatPlaytime, localImage, type Recap } from "./library";

  let {
    appid,
    imagesDir,
    onclose,
    onshowcase,
  }: {
    appid: number;
    imagesDir: string;
    onclose: () => void;
    /** Absent quand on est déjà dans la vitrine. */
    onshowcase?: () => void;
  } = $props();

  let recap = $state<Recap | null>(null);
  let error = $state("");
  let canvas: HTMLCanvasElement;
  let closeButton = $state<HTMLButtonElement>();

  const cover = $derived(recap ? localImage(imagesDir, recap.game.cover_file) : null);

  onMount(() => {
    let stop: (() => void) | null = null;
    let dead = false;
    api
      .recap(appid)
      .then(async (r) => {
        recap = r;
        closeButton?.focus();
        if (reduced()) return;
        const colors = await coverPalette(localImage(imagesDir, r.game.cover_file));
        if (!dead) stop = rain(canvas, colors);
      })
      .catch((e) => (error = String(e)));
    return () => {
      dead = true;
      stop?.();
    };
  });
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div
  class="fixed inset-0 z-50 grid place-items-center bg-black/70 p-4 backdrop-blur-sm"
  role="presentation"
  transition:backdrop
  onclick={(e) => e.target === e.currentTarget && onclose()}
>
  {#if error}
    <p class="rounded-lg bg-panel p-4 text-sm text-danger">{error}</p>
  {:else if recap}
    {@const g = recap.game}
    {@const r = recap.rarest}
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="celebration-title"
      class="pop-in flex w-full max-w-lg flex-col items-center gap-5 rounded-2xl border border-gold/40 bg-panel px-6 pt-8 pb-6 text-center shadow-2xl shadow-gold/10 sm:flex-row sm:items-stretch sm:text-left"
    >
      <!-- Jaquette dorée -->
      <div class="gold-frame w-36 shrink-0 self-center rounded-xl bg-raised" style:aspect-ratio="2 / 3">
        {#if cover}
          <img src={cover} alt="" class="h-full w-full object-cover {g.cover_kind === 'header' ? 'object-left' : ''}" />
        {:else}
          <div class="grid h-full place-items-center text-5xl" aria-hidden="true">🏆</div>
        {/if}
        <span class="absolute top-2 right-2 z-10 rounded-full bg-gold px-2 py-0.5 text-xs font-bold text-bg shadow">100 %</span>
      </div>

      <div class="flex min-w-0 flex-1 flex-col">
        <p class="text-xs font-semibold tracking-[0.2em] text-gold uppercase">Jeu terminé</p>
        <h2 id="celebration-title" class="mt-1 text-2xl leading-tight font-bold">{g.name}</h2>
        <p class="mt-1 text-sm text-muted">{g.unlocked} / {g.total} succès obtenus</p>

        <dl class="mt-4 space-y-2.5 text-sm">
          {#if g.completed_at}
            <div>
              <dt class="text-xs text-muted">100 % le</dt>
              <dd class="font-medium">{formatDate(g.completed_at)}</dd>
            </div>
          {/if}
          {#if g.playtime_minutes}
            <div>
              <dt class="text-xs text-muted">Temps de jeu Steam</dt>
              <dd class="font-medium">{formatPlaytime(g.playtime_minutes)}</dd>
            </div>
          {/if}
          {#if r}
            <div>
              <dt class="text-xs text-muted">Succès le plus rare</dt>
              <dd class="mt-0.5 flex items-center justify-center gap-2 sm:justify-start">
                {#if r.icon_url}<img src={r.icon_url} alt="" class="h-8 w-8 shrink-0 rounded" />{/if}
                <span class="min-w-0">
                  <span class="block truncate font-medium">{r.name}</span>
                  <span class="text-xs {rarityText(r.rarity)}">{r.rarity?.toFixed(1)} % des joueurs</span>
                </span>
              </dd>
            </div>
          {/if}
        </dl>

        <div class="mt-auto flex justify-center gap-2 pt-5 sm:justify-end">
          {#if onshowcase}
            <button onclick={onshowcase} class="rounded-lg border border-line px-3 py-1.5 text-sm hover:bg-raised">
              Voir la vitrine
            </button>
          {/if}
          <button
            bind:this={closeButton}
            onclick={onclose}
            class="rounded-lg bg-gold px-4 py-1.5 text-sm font-semibold text-bg hover:brightness-110"
          >
            Continuer
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<!-- Au-dessus de tout, sans intercepter la souris -->
<canvas bind:this={canvas} class="pointer-events-none fixed inset-0 z-[60] h-full w-full" aria-hidden="true"></canvas>
