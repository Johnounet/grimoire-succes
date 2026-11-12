<script lang="ts">
  import { localImage } from "../library";
  import { imageFrom, saveBlock, uploadImage } from "./store.svelte";
  import type { Block } from "./types";

  let { block, imagesDir }: { block: Block<"image">; imagesDir: string } = $props();

  let busy = $state(false);
  let error = $state("");
  let over = $state(false);
  let zoom = $state(false);
  let fileInput = $state<HTMLInputElement>();

  const src = $derived(localImage(imagesDir, block.data.file));

  async function take(file: Blob | null) {
    if (!file) return;
    busy = true;
    error = "";
    try {
      block.data.file = await uploadImage(file);
      saveBlock(block);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function paste(e: ClipboardEvent) {
    const f = imageFrom(e.clipboardData);
    if (f) {
      e.preventDefault();
      e.stopPropagation();
      take(f);
    }
  }

  function drop(e: DragEvent) {
    over = false;
    const f = imageFrom(e.dataTransfer);
    if (f) {
      e.preventDefault();
      e.stopPropagation();
      take(f);
    }
  }

  function dragover(e: DragEvent) {
    if (e.dataTransfer?.types.includes("Files")) {
      e.preventDefault();
      e.stopPropagation();
      over = true;
    }
  }
</script>

<input
  bind:this={fileInput}
  type="file"
  accept="image/png,image/jpeg,image/gif,image/webp,image/bmp"
  class="hidden"
  onchange={(e) => take(e.currentTarget.files?.[0] ?? null)}
/>

{#if src}
  <figure class="space-y-1.5" ondragover={dragover} ondragleave={() => (over = false)} ondrop={drop}>
    <button
      onclick={() => (zoom = true)}
      class="block w-full overflow-hidden rounded-lg bg-bg {over ? 'ring-2 ring-accent' : ''}"
      title="Agrandir"
    >
      <img src={src} alt={block.data.caption || "Image du guide"} class="mx-auto max-h-80 w-auto object-contain" />
    </button>
    <figcaption class="flex items-center gap-2">
      <input
        bind:value={block.data.caption}
        oninput={() => saveBlock(block)}
        placeholder="Légende (facultatif)"
        aria-label="Légende de l'image"
        class="min-w-0 flex-1 bg-transparent text-xs text-muted outline-none placeholder:text-muted/60 focus:text-fg"
      />
      <button onclick={() => fileInput?.click()} class="shrink-0 text-xs text-muted underline hover:text-fg">remplacer</button>
    </figcaption>
  </figure>
{:else}
  <div
    role="button"
    tabindex="0"
    onpaste={paste}
    ondragover={dragover}
    ondragleave={() => (over = false)}
    ondrop={drop}
    onclick={() => fileInput?.click()}
    onkeydown={(e) => e.key === "Enter" && fileInput?.click()}
    class="grid min-h-28 cursor-pointer place-items-center rounded-lg border-2 border-dashed p-4 text-center text-sm transition
      focus-visible:border-accent focus-visible:outline-none
      {over ? 'border-accent bg-accent/10 text-fg' : 'border-line text-muted hover:border-muted'}"
  >
    {#if busy}
      Enregistrement de l'image…
    {:else}
      <span>
        <strong class="text-fg">Glissez une image</strong>, cliquez pour choisir un fichier,<br />
        ou sélectionnez cette zone puis <kbd class="rounded border border-line px-1">Ctrl</kbd>+<kbd class="rounded border border-line px-1">V</kbd>
      </span>
    {/if}
  </div>
{/if}

{#if error}<p class="mt-1 text-xs text-danger">{error}</p>{/if}

{#if zoom && src}
  <div
    role="dialog"
    aria-modal="true"
    aria-label="Image agrandie"
    tabindex="-1"
    onclick={() => (zoom = false)}
    onkeydown={(e) => e.key === "Escape" && (zoom = false)}
    class="fixed inset-0 z-50 grid cursor-zoom-out place-items-center bg-black/85 p-6"
    {@attach (el) => el.focus()}
  >
    <figure class="max-h-full max-w-full space-y-2 text-center">
      <img src={src} alt={block.data.caption || "Image du guide"} class="max-h-[85vh] max-w-full rounded-lg object-contain" />
      {#if block.data.caption}<figcaption class="text-sm text-fg/80">{block.data.caption}</figcaption>{/if}
    </figure>
  </div>
{/if}
