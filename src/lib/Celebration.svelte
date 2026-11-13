<script lang="ts">
  // Célébration du 100 % : le récap (date du 100 %, temps de jeu Steam, succès le plus rare) dans une
  // boîte entourée d'un éclat doré (liseré qui tourne, halo qui respire, reflet à l'apparition,
  // `.gold-aura` dans app.css). Rejouable depuis le badge « 100 % » du jeu.
  // Un jeu fini est fait pour être retiré : boutons « Exporter le guide » (pour le garder) et « Retirer le jeu ».
  import ExportDialog from "./ExportDialog.svelte";
  import { backdrop } from "./motion.svelte";
  import { onMount } from "svelte";
  import { rarityText } from "./filters";
  import { api, formatDate, formatPlaytime, localImage, type Recap } from "./library";

  let {
    appid,
    imagesDir,
    onclose,
    onremoved,
  }: {
    appid: number;
    imagesDir: string;
    onclose: () => void;
    /** Le jeu a été retiré de la bibliothèque. */
    onremoved: (appid: number) => void;
  } = $props();

  let exporting = $state(false);
  let confirmRemove = $state(false);
  let removing = $state(false);

  async function remove() {
    removing = true;
    try {
      await api.deleteGame(appid);
      onremoved(appid);
    } catch (e) {
      error = `Suppression impossible : ${e}`;
      removing = false;
    }
  }

  let recap = $state<Recap | null>(null);
  let error = $state("");
  let closeButton = $state<HTMLButtonElement>();

  const cover = $derived(recap ? localImage(imagesDir, recap.game.cover_file) : null);

  onMount(() => {
    api
      .recap(appid)
      .then((r) => {
        recap = r;
        // Après le rendu de la boîte
        queueMicrotask(() => closeButton?.focus());
      })
      .catch((e) => (error = String(e)));
  });
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && !exporting && onclose()} />

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
    <div class="gold-aura pop-in w-full max-w-lg rounded-2xl">
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="celebration-title"
      class="gold-aura-box relative flex flex-col items-center gap-5 overflow-hidden rounded-2xl bg-panel px-6 pt-8 pb-6 text-center shadow-2xl sm:flex-row sm:items-stretch sm:text-left"
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

        {#if confirmRemove}
          <div class="mt-5 rounded-lg border border-danger/40 bg-danger/10 p-3 text-sm">
            <p>Retirer « {g.name} » ? Son espace de travail et ses guides seront effacés.</p>
            <div class="mt-2 flex flex-wrap justify-center gap-2 sm:justify-end">
              <button onclick={() => (confirmRemove = false)} class="rounded-md px-3 py-1 hover:bg-raised">Annuler</button>
              <button onclick={() => (exporting = true)} class="rounded-md border border-line px-3 py-1 hover:bg-raised">
                Exporter d'abord…
              </button>
              <button onclick={remove} disabled={removing} class="rounded-md bg-danger px-3 py-1 font-medium text-white disabled:opacity-60">
                Retirer
              </button>
            </div>
          </div>
        {:else}
          <div class="mt-auto flex flex-col items-center gap-2 pt-5 sm:items-end">
            <button
              bind:this={closeButton}
              onclick={onclose}
              class="rounded-lg bg-gold px-4 py-1.5 text-sm font-semibold text-bg hover:brightness-110"
            >
              Continuer
            </button>
            <div class="flex gap-3 text-xs text-muted">
              <button onclick={() => (exporting = true)} class="underline-offset-2 hover:text-fg hover:underline">
                Exporter le guide…
              </button>
              <button onclick={() => (confirmRemove = true)} class="underline-offset-2 hover:text-danger hover:underline">
                Retirer le jeu…
              </button>
            </div>
          </div>
        {/if}
      </div>
    </div>
    </div>
  {/if}
</div>

{#if exporting && recap}
  <ExportDialog {appid} gameName={recap.game.name} onclose={() => (exporting = false)} />
{/if}
