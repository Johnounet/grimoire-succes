<script lang="ts">
  // Menu « ⋯ » discret : quelques actions rares (re-lock, épingle…).
  import { pop } from "./motion.svelte";
  import { popoverStyle } from "./popover";
  export interface MenuItem {
    label: string;
    onclick: () => void;
    hint?: string;
  }

  let {
    items,
    label = "Plus d'actions",
    class: klass = "",
    buttonClass = "",
  }: {
    items: MenuItem[];
    label?: string;
    class?: string;
    /** Style du bouton « ⋯ ». Un flou d'arrière-plan (backdrop-blur), un filtre ou une transformation se met
     *  ici, jamais dans `class` : sur un ancêtre du menu, il piégerait sa position fixe (voir popover.ts). */
    buttonClass?: string;
  } = $props();
  let open = $state(false);
  let button: HTMLButtonElement | undefined = $state();
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
    class="grid h-7 w-7 place-items-center rounded text-muted transition hover:bg-raised hover:text-fg {buttonClass}"
  >
    ⋯
  </button>
  {#if open}
    <div
      role="menu"
      tabindex="-1"
      transition:pop={{ y: -4 }}
      onkeydown={(e) => {
        if (e.key === "Escape") open = false;
      }}
      style={popoverStyle(button, 256, "right")}
      class="z-50 rounded-lg border border-line bg-raised p-1 shadow-xl"
    >
      {#each items as it, i (it.label)}
        <button
          role="menuitem"
          onclick={() => {
            open = false;
            it.onclick();
          }}
          {@attach (el) => {
            if (i === 0) el.focus();
          }}
          class="block w-full rounded-md px-2.5 py-1.5 text-left text-sm hover:bg-panel focus-visible:bg-panel focus-visible:outline-none"
        >
          {it.label}
          {#if it.hint}<span class="block text-[11px] text-muted">{it.hint}</span>{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>
