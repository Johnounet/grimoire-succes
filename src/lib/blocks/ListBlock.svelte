<script lang="ts">
  import { tick } from "svelte";
  import { saveBlock } from "./store.svelte";
  import type { Block } from "./types";

  let { block }: { block: Block<"list"> } = $props();

  let draft = $state("");
  let inputs: HTMLInputElement[] = $state([]);

  const items = $derived(block.data.items);
  const done = $derived(items.filter((i) => i.checked).length);
  const pct = $derived(items.length ? (done / items.length) * 100 : 0);

  function add() {
    const text = draft.trim();
    if (!text) return;
    block.data.items.push({ text, checked: false });
    draft = "";
    saveBlock(block);
  }

  async function insertAfter(i: number) {
    block.data.items.splice(i + 1, 0, { text: "", checked: false });
    saveBlock(block);
    await tick();
    inputs[i + 1]?.focus();
  }

  async function removeAt(i: number, focusPrev = false) {
    block.data.items.splice(i, 1);
    saveBlock(block);
    if (focusPrev) {
      await tick();
      inputs[Math.max(0, i - 1)]?.focus();
    }
  }
</script>

<div class="space-y-1.5">
  <div class="flex items-center gap-2 text-xs text-muted">
    <span class="font-semibold {items.length && done === items.length ? 'text-ok' : 'text-fg'}">{done} / {items.length}</span>
    <div class="h-1 flex-1 overflow-hidden rounded-full bg-line" aria-hidden="true">
      <div class="h-full rounded-full bg-ok transition-[width] duration-300" style="width: {pct}%"></div>
    </div>
    {#if done > 0}
      <button
        onclick={() => {
          block.data.items.forEach((i) => (i.checked = false));
          saveBlock(block);
        }}
        class="underline hover:text-fg">tout décocher</button
      >
    {/if}
  </div>

  <ul class="space-y-0.5">
    {#each block.data.items as item, i (i)}
      <li class="group/item flex items-center gap-2 rounded px-1 hover:bg-raised/40">
        <input
          type="checkbox"
          bind:checked={item.checked}
          onchange={() => saveBlock(block)}
          aria-label={`Cocher « ${item.text || "élément vide"} »`}
          class="h-4 w-4 shrink-0 cursor-pointer accent-[var(--color-ok)]"
        />
        <input
          bind:this={inputs[i]}
          bind:value={item.text}
          oninput={() => saveBlock(block)}
          onkeydown={(e) => {
            if (e.key === "Enter") (e.preventDefault(), insertAfter(i));
            else if (e.key === "Backspace" && item.text === "") (e.preventDefault(), removeAt(i, true));
          }}
          aria-label="Texte de l'élément"
          class="min-w-0 flex-1 bg-transparent py-1 text-sm outline-none {item.checked ? 'text-muted line-through' : ''}"
        />
        <button
          onclick={() => removeAt(i)}
          aria-label={`Supprimer « ${item.text || "élément vide"} »`}
          class="text-xs text-muted opacity-0 group-hover/item:opacity-100 hover:text-danger focus:opacity-100"
        >
          ✕
        </button>
      </li>
    {/each}
  </ul>

  <form
    onsubmit={(e) => {
      e.preventDefault();
      add();
    }}
    class="flex items-center gap-2 px-1"
  >
    <span class="text-muted" aria-hidden="true">+</span>
    <input
      bind:value={draft}
      placeholder="Ajouter un élément (Entrée)"
      aria-label="Nouvel élément"
      class="min-w-0 flex-1 bg-transparent py-1 text-sm outline-none placeholder:text-muted/70"
    />
  </form>
</div>
