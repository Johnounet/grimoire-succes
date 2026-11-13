<script lang="ts">
  import { counterAdd } from "./ops";
  import { saveBlock } from "./store.svelte";
  import type { Block } from "./types";

  let { block }: { block: Block<"counter"> } = $props();

  const d = $derived(block.data);
  const reached = $derived(d.goal != null && d.value >= d.goal);
  const pct = $derived(d.goal ? Math.min(100, Math.max(0, (d.value / d.goal) * 100)) : 0);
  let bump = $state(0);

  function add(sign: 1 | -1) {
    counterAdd(block.data, sign);
    bump++;
    saveBlock(block);
  }

  function setNumber(field: "value" | "goal" | "step", raw: string) {
    const n = Math.floor(Number(raw));
    if (field === "goal") block.data.goal = raw.trim() === "" || !Number.isFinite(n) || n <= 0 ? null : n;
    else if (Number.isFinite(n)) block.data[field] = field === "step" ? Math.max(1, n) : Math.max(0, n);
    saveBlock(block);
  }
</script>

<div class="space-y-2">
  <input
    bind:value={block.data.label}
    oninput={() => saveBlock(block)}
    placeholder="Ce que vous comptez (ex. plumes trouvées)"
    aria-label="Nom du compteur"
    class="w-full bg-transparent text-sm text-muted outline-none placeholder:text-muted/60 focus:text-fg"
  />

  <div class="flex items-center gap-3">
    <button
      onclick={() => add(-1)}
      disabled={d.value <= 0}
      aria-label={`Retirer ${d.step}`}
      class="h-10 w-10 shrink-0 rounded-lg border border-line text-lg font-semibold transition hover:bg-raised disabled:opacity-40"
    >
      −
    </button>

    <div class="min-w-0 flex-1 text-center">
      <div class="flex items-baseline justify-center gap-1 tabular-nums">
        {#key bump}
          <input
            value={d.value}
            onchange={(e) => setNumber("value", e.currentTarget.value)}
            inputmode="numeric"
            aria-label="Valeur actuelle"
            class="bump w-20 bg-transparent text-right text-3xl font-bold outline-none {reached ? 'text-ok' : ''}"
          />
        {/key}
        <span class="text-xl text-muted">/</span>
        <input
          value={d.goal ?? ""}
          onchange={(e) => setNumber("goal", e.currentTarget.value)}
          inputmode="numeric"
          placeholder="∞"
          aria-label="Objectif"
          class="w-16 bg-transparent text-xl text-muted outline-none placeholder:text-muted/50 focus:text-fg"
        />
      </div>
      {#if d.goal}
        <div class="mt-1.5 h-1.5 overflow-hidden rounded-full bg-line" aria-hidden="true">
          <div class="h-full rounded-full transition-[width] duration-300 {reached ? 'bg-ok' : 'bg-accent'}" style="width: {pct}%"></div>
        </div>
      {/if}
    </div>

    <button
      onclick={() => add(1)}
      aria-label={`Ajouter ${d.step}`}
      class="h-10 w-10 shrink-0 rounded-lg bg-accent text-lg font-semibold text-on-accent transition hover:brightness-110"
    >
      +
    </button>
  </div>

  <div class="flex items-center justify-end gap-1 text-xs text-muted">
    <label for="step-{block.id}">Pas</label>
    <input
      id="step-{block.id}"
      value={d.step}
      onchange={(e) => setNumber("step", e.currentTarget.value)}
      inputmode="numeric"
      class="w-10 rounded border border-line bg-bg px-1 text-center outline-none focus:border-accent"
    />
    {#if reached}<span class="ml-2 font-semibold text-ok">Objectif atteint ✓</span>{/if}
  </div>
</div>

<style>
  .bump {
    animation: bump 0.25s ease-out;
  }
  @keyframes bump {
    50% {
      transform: scale(1.12);
    }
  }
</style>
