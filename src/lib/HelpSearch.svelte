<script lang="ts">
  // Bouton « Aide » d'un succès, toujours visible : menu YouTube, guides Steam, Google (navigateur par défaut).
  import { HELP_TARGETS, helpGame, openHelp } from "./helpSearch";
  import MoreMenu from "./MoreMenu.svelte";

  let { name }: { name: string } = $props();

  const game = helpGame();
  // Jeu et nom lus au clic : rien n'est recalculé quand le jeu change (une coche, par exemple)
  const items = game
    ? HELP_TARGETS.map((t) => ({ label: t.label, hint: t.hint, onclick: () => openHelp(t.url(game(), name)) }))
    : [];
</script>

{#if items.length}
  <MoreMenu
    {items}
    label="Chercher de l'aide sur ce succès"
    buttonClass="inline-flex items-center gap-1 rounded-full border border-line px-1.5 py-px text-[10px] leading-4 text-muted
      transition hover:border-muted hover:text-fg focus-visible:outline-2 focus-visible:outline-accent"
  >
    {#snippet trigger()}
      <svg viewBox="0 0 16 16" class="h-3 w-3" aria-hidden="true">
        <circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" stroke-width="1.8" />
        <path d="M10.4 10.4L14 14" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
      </svg>
      Aide
    {/snippet}
  </MoreMenu>
{/if}
