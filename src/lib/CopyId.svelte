<script lang="ts">
  // ID technique d'un succès, copié dans le presse-papiers en un clic.
  let { id }: { id: string } = $props();

  let copied = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function copy() {
    try {
      await navigator.clipboard.writeText(id);
      copied = true;
      clearTimeout(timer);
      timer = setTimeout(() => (copied = false), 1500);
    } catch {
      copied = false;
    }
  }
</script>

<button
  onclick={copy}
  class="group/copy mt-0.5 inline-flex max-w-full items-center gap-1 rounded px-1 font-mono text-[11px] text-muted/70
    transition hover:bg-raised hover:text-fg focus-visible:outline-2 focus-visible:outline-accent"
  title="Copier l'ID technique"
>
  <span class="truncate">{id}</span>
  <span class="shrink-0 {copied ? 'text-ok' : 'opacity-0 group-hover/copy:opacity-100'}">{copied ? "✓ copié" : "⧉"}</span>
</button>
