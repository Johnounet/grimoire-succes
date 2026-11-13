<script lang="ts">
  // Vidéo d'une carte épinglée, dans la fenêtre compacte : une ligne, et le lecteur une fois lancée
  // (lecture dès l'ouverture). Un seul lecteur ouvert à la fois : c'est la fenêtre compacte qui décide
  // (`open`). Actions des raccourcis : `right` (lancer, lecture / pause) et `left` (recul de 10 s).
  import { embedUrl, parseYouTube } from "../youtube";
  import { YouTubePlayer } from "./player.svelte";
  import { formatClock, type Block } from "./types";

  let {
    block,
    open,
    onopen,
    onclose,
  }: {
    block: Block<"video">;
    open: boolean;
    onopen: () => void;
    onclose: () => void;
  } = $props();

  const BACK_SECONDS = 10;

  // svelte-ignore state_referenced_locally (l'id d'un bloc ne change pas)
  const player = new YouTubePlayer(block.id);
  const ref = $derived(parseYouTube(block.data.url));
  const markers = $derived([...block.data.markers].sort((a, b) => a.time - b.time));

  $effect(() => player.attach());
  $effect(() => {
    if (!open) player.playing = false;
  });

  /** Action Droite : ouvre et lance la vidéo, ou lecture / pause. Renvoie le texte du retour visuel. */
  export function right(): { ok: boolean; text: string } {
    if (!ref) return { ok: false, text: "Vidéo sans lien YouTube" };
    if (!open) {
      onopen();
      return { ok: true, text: "Vidéo lancée" };
    }
    const was = player.playing;
    player.toggle();
    return { ok: true, text: was ? "Vidéo en pause" : "Lecture" };
  }

  /** Action Gauche : recule de quelques secondes. */
  export function left(): { ok: boolean; text: string } {
    if (!open) return { ok: false, text: "Vidéo fermée : Droite pour la lancer" };
    player.back(BACK_SECONDS);
    return { ok: true, text: `Recul de ${BACK_SECONDS} s (${formatClock(player.current)})` };
  }
</script>

<div class="space-y-1.5">
  <div class="flex items-center gap-2">
    <div class="min-w-0 flex-1">
      <div class="truncate text-[11px] text-muted">▶ Vidéo</div>
      <div class="truncate text-sm">
        {#if !ref}
          <span class="text-muted">sans lien</span>
        {:else if open}
          <span class="font-mono tabular-nums">{formatClock(player.current)}</span>
          <span class="text-xs text-muted">· {player.playing ? "lecture" : "pause"}</span>
        {:else}
          <span class="text-muted">{markers.length ? `${markers.length} repère${markers.length > 1 ? "s" : ""}` : "YouTube"}</span>
        {/if}
      </div>
    </div>
    <button
      onclick={right}
      disabled={!ref}
      class="h-8 w-8 rounded-md font-semibold hover:brightness-110 disabled:opacity-40 {open && player.playing ? 'bg-danger text-white' : 'bg-accent text-on-accent'}"
      aria-label={open && player.playing ? "Pause" : "Lecture"}
      title={open && player.playing ? "Pause" : "Lecture"}
    >
      {open && player.playing ? "❚❚" : "▶"}
    </button>
    {#if open}
      <button onclick={onclose} class="h-8 w-8 rounded-md border border-line text-sm text-muted hover:bg-raised" aria-label="Fermer la vidéo" title="Fermer la vidéo">✕</button>
    {/if}
  </div>

  {#if open && ref}
    <div class="aspect-video overflow-hidden rounded-md bg-black">
      <iframe
        bind:this={player.iframe}
        onload={() => player.loaded()}
        src={embedUrl(ref, true)}
        title="Vidéo YouTube"
        class="h-full w-full"
        allow="autoplay; encrypted-media; picture-in-picture; fullscreen"
        referrerpolicy="strict-origin-when-cross-origin"
        allowfullscreen
      ></iframe>
    </div>
    {#if markers.length}
      <div class="flex flex-wrap gap-1">
        {#each markers as m (m)}
          <button
            onclick={() => player.seek(m.time)}
            title={`Aller à ${formatClock(m.time)}`}
            class="max-w-full truncate rounded bg-raised px-1.5 py-0.5 text-[11px] hover:bg-accent/20"
          >
            <span class="font-mono text-accent tabular-nums">{formatClock(m.time)}</span> {m.label}
          </button>
        {/each}
      </div>
    {/if}
  {/if}
</div>
