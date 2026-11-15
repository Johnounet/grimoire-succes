<script lang="ts">
  // Menu « ⋯ » discret : quelques actions rares (re-lock, épingle…).
  import type { Snippet } from "svelte";
  import { pop } from "./motion.svelte";
  import { popoverStyle } from "./popover";
  export interface MenuItem {
    label: string;
    onclick: () => void;
    hint?: string;
    /** Titre du groupe : affiché (avec un trait) avant le premier élément d'un nouveau groupe, et les
     *  éléments du groupe sont en retrait. */
    group?: string;
    /** Pastille de couleur (valeur CSS) devant le libellé, ex. couleur d'une section. */
    color?: string;
  }

  let {
    items,
    label = "Plus d'actions",
    class: klass = "",
    buttonClass = "",
    trigger,
  }: {
    items: MenuItem[];
    label?: string;
    class?: string;
    /** Style du bouton « ⋯ ». Un flou d'arrière-plan (backdrop-blur), un filtre ou une transformation se met
     *  ici, jamais dans `class` : sur un ancêtre du menu, il piégerait sa position fixe (voir popover.ts). */
    buttonClass?: string;
    /** Contenu du bouton à la place de « ⋯ » ; `buttonClass` donne alors tout son style. */
    trigger?: Snippet;
  } = $props();
  let open = $state(false);
  let button: HTMLButtonElement | undefined = $state();
  /** Hauteur du menu ouvert, mesurée pour le placer du côté où il tient (popover.ts). */
  let menuHeight = $state(0);
</script>

<div
  class="relative {klass}"
  onfocusout={(e) => {
    if (!e.currentTarget.contains(e.relatedTarget as Node | null)) open = false;
  }}
>
  <button
    bind:this={button}
    onclick={() => (open = !open)}
    aria-expanded={open}
    aria-haspopup="menu"
    aria-label={label}
    title={label}
    class={trigger
      ? buttonClass
      : `grid h-7 w-7 place-items-center rounded text-muted transition hover:bg-raised hover:text-fg ${buttonClass}`}
  >
    {#if trigger}{@render trigger()}{:else}⋯{/if}
  </button>
  {#if open}
    <div
      role="menu"
      tabindex="-1"
      transition:pop={{ y: -4 }}
      onkeydown={(e) => {
        if (e.key === "Escape") open = false;
      }}
      style={popoverStyle(button, 256, "right", menuHeight)}
      {@attach (el) => {
        menuHeight = el.scrollHeight;
        return () => (menuHeight = 0);
      }}
      class="z-50 rounded-lg border border-line bg-raised p-1 shadow-xl"
    >
      {#each items as it, i (i)}
        {#if it.group && it.group !== items[i - 1]?.group}
          <div
            role="presentation"
            class="px-2.5 pt-1.5 pb-0.5 text-[11px] font-semibold tracking-wide text-muted uppercase
              {i > 0 ? 'mt-1 border-t border-line' : ''}"
          >
            {it.group}
          </div>
        {/if}
        <button
          role="menuitem"
          onclick={() => {
            open = false;
            it.onclick();
          }}
          {@attach (el) => {
            if (i === 0) el.focus();
          }}
          class="block w-full rounded-md py-1.5 pr-2.5 text-left text-sm hover:bg-panel focus-visible:bg-panel focus-visible:outline-none
            {it.group ? 'pl-4' : 'pl-2.5'}"
        >
          {#if it.color}<span class="mr-1.5 inline-block h-2.5 w-2.5 rounded-sm align-[-1px]" style:background={it.color}></span>{/if}
          {it.label}
          {#if it.hint}<span class="block text-[11px] text-muted">{it.hint}</span>{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>
