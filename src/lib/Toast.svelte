<script lang="ts">
  // Notification en bas de l'écran, avec « Annuler » quand c'est possible.
  import { pop } from "./motion.svelte";
  import { dismissToast, toast } from "./blocks/store.svelte";
</script>

{#if toast.message}
  <div
    role="status"
    transition:pop={{ y: 12 }}
    class="fixed bottom-5 left-1/2 z-40 flex -translate-x-1/2 items-center gap-4 rounded-xl border border-line bg-raised px-4 py-2.5 text-sm shadow-2xl"
  >
    <span>{toast.message}</span>
    {#if toast.undo}
      <button
        onclick={() => {
          const undo = toast.undo;
          dismissToast();
          undo?.();
        }}
        class="font-semibold text-accent hover:underline">Annuler</button
      >
    {/if}
    <button onclick={dismissToast} class="text-muted hover:text-fg" aria-label="Fermer la notification">✕</button>
  </div>
{/if}
