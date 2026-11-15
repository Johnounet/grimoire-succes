<script lang="ts">
  // Bouton « Aide » d'un succès, toujours visible : menu YouTube, guides Steam, Google (navigateur par défaut).
  import { HELP_TARGETS, helpGame, openHelp } from "./helpSearch";
  import MoreMenu from "./MoreMenu.svelte";

  /** `iconOnly` : simple loupe (en-tête des cartes de l'espace de travail, à droite du nom). */
  let { name, iconOnly = false }: { name: string; iconOnly?: boolean } = $props();

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
    buttonClass={iconOnly
      ? "grid h-5 w-5 shrink-0 place-items-center rounded text-muted transition hover:bg-raised hover:text-fg focus-visible:outline-2 focus-visible:outline-accent"
      : "inline-flex items-center gap-1 rounded-full border border-line px-1.5 py-px text-[10px] leading-4 text-muted transition hover:border-muted hover:text-fg focus-visible:outline-2 focus-visible:outline-accent"}
  >
    {#snippet trigger()}
      <svg viewBox="0 0 16 16" class={iconOnly ? "h-3.5 w-3.5" : "h-3 w-3"} aria-hidden="true">
        <circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" stroke-width="1.8" />
        <path d="M10.4 10.4L14 14" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
      </svg>
      {#if !iconOnly}Aide{/if}
    {/snippet}
  </MoreMenu>
{/if}
