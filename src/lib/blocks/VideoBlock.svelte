<script lang="ts">
  // Vidéo YouTube intégrée (youtube-nocookie.com) avec repères de temps.
  // Lecteur piloté par postMessage (voir player.svelte.ts).
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { tick } from "svelte";
  import { FIT_CARD_EVENT } from "../library";
  import { embedUrl, parseYouTube, watchUrl } from "../youtube";
  import { YouTubePlayer } from "./player.svelte";
  import { saveBlock } from "./store.svelte";
  import { formatClock, parseClock, type Block } from "./types";

  let { block }: { block: Block<"video"> } = $props();

  let urlDraft = $state("");
  let urlError = $state("");
  // svelte-ignore state_referenced_locally (l'id d'un bloc ne change pas)
  const player = new YouTubePlayer(block.id);
  const current = $derived(player.current);
  let timeDraft = $state("");
  let labelDraft = $state("");
  let markerError = $state("");

  const ref = $derived(parseYouTube(block.data.url));
  const markers = $derived([...block.data.markers].sort((a, b) => a.time - b.time));

  async function setUrl() {
    const r = parseYouTube(urlDraft);
    if (!r) {
      urlError = "Lien YouTube non reconnu.";
      return;
    }
    block.data.url = urlDraft.trim();
    urlDraft = "";
    urlError = "";
    saveBlock(block);
    // Tableau : la carte s'agrandit pour montrer toute la vidéo (jamais ne rapetisse)
    await tick();
    player.iframe?.dispatchEvent(new CustomEvent(FIT_CARD_EVENT, { bubbles: true }));
  }

  const seek = (t: number) => player.seek(t);

  // Position et état de lecture, pause quand la même vidéo démarre dans la fenêtre compacte
  $effect(() => player.attach());

  function addMarker(e: SubmitEvent) {
    e.preventDefault();
    const t = timeDraft.trim() === "" ? Math.floor(current) : parseClock(timeDraft);
    if (t == null) {
      markerError = "Temps illisible : écrivez par exemple 3:42.";
      return;
    }
    block.data.markers.push({ time: t, label: labelDraft.trim() || "Repère" });
    timeDraft = "";
    labelDraft = "";
    markerError = "";
    saveBlock(block);
  }

  function removeMarker(m: { time: number; label: string }) {
    const i = block.data.markers.indexOf(m);
    if (i >= 0) block.data.markers.splice(i, 1);
    saveBlock(block);
  }

  function open(at = 0) {
    if (ref) openUrl(watchUrl(ref, at)).catch(() => {});
  }
</script>

{#if !ref}
  <form onsubmit={(e) => (e.preventDefault(), setUrl())} class="space-y-1.5">
    <div class="flex gap-2">
      <input
        bind:value={urlDraft}
        placeholder="Collez un lien YouTube (youtube.com/watch?v=…, youtu.be/…)"
        aria-label="Lien de la vidéo YouTube"
        class="min-w-0 flex-1 rounded-lg border border-line bg-bg px-3 py-1.5 text-sm outline-none focus:border-accent"
      />
      <button class="shrink-0 rounded-lg bg-accent px-3 py-1.5 text-sm font-medium text-white hover:brightness-110">OK</button>
    </div>
    {#if urlError}<p class="text-xs text-danger">{urlError}</p>{/if}
  </form>
{:else}
  <div class="space-y-2">
    <div class="aspect-video overflow-hidden rounded-lg bg-black">
      <iframe
        bind:this={player.iframe}
        onload={() => player.loaded()}
        src={embedUrl(ref)}
        title="Vidéo YouTube"
        class="h-full w-full"
        allow="autoplay; encrypted-media; picture-in-picture; fullscreen"
        referrerpolicy="strict-origin-when-cross-origin"
        allowfullscreen
      ></iframe>
    </div>
    <div class="flex flex-wrap items-center gap-3 text-xs text-muted">
      <button onclick={() => open(current)} class="underline hover:text-fg">Ouvrir sur YouTube</button>
      <span class="opacity-70">La vidéo ne se lance pas ? Le lien ouvre votre navigateur.</span>
      <button
        onclick={() => {
          block.data.url = "";
          saveBlock(block);
        }}
        class="ml-auto underline hover:text-fg">changer de vidéo</button
      >
    </div>

    <!-- Repères -->
    {#if markers.length}
      <ul class="space-y-0.5">
        {#each markers as m (m)}
          <li class="group/m flex items-center gap-2 rounded px-1 hover:bg-raised/40">
            <button onclick={() => seek(m.time)} class="flex min-w-0 flex-1 items-center gap-2 py-1 text-left text-sm" title="Aller à ce moment">
              <span class="shrink-0 rounded bg-accent/15 px-1.5 font-mono text-xs text-accent tabular-nums">{formatClock(m.time)}</span>
              <span class="text-muted" aria-hidden="true">→</span>
              <span class="truncate">{m.label}</span>
            </button>
            <button
              onclick={() => removeMarker(m)}
              aria-label={`Supprimer le repère « ${m.label} »`}
              class="text-xs text-muted opacity-0 group-hover/m:opacity-100 hover:text-danger focus:opacity-100">✕</button
            >
          </li>
        {/each}
      </ul>
    {/if}
    <form onsubmit={addMarker} class="flex items-center gap-2">
      <input
        bind:value={timeDraft}
        placeholder={formatClock(current)}
        aria-label="Temps du repère (vide : position actuelle)"
        title="Laisser vide pour prendre la position actuelle de la vidéo"
        class="w-16 rounded border border-line bg-bg px-1.5 py-1 text-center font-mono text-xs outline-none focus:border-accent"
      />
      <input
        bind:value={labelDraft}
        placeholder="Nouveau repère (ex. le coffre caché)"
        aria-label="Libellé du repère"
        class="min-w-0 flex-1 rounded border border-line bg-bg px-2 py-1 text-xs outline-none focus:border-accent"
      />
      <button class="shrink-0 rounded border border-line px-2 py-1 text-xs hover:bg-raised">+ Repère</button>
    </form>
    {#if markerError}<p class="text-xs text-danger">{markerError}</p>{/if}
  </div>
{/if}
