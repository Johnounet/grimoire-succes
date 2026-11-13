<script lang="ts">
  // Chronomètre : Démarrer → Arrêter (la course est terminée, le meilleur temps est mis à jour).
  // L'heure de départ est enregistrée : un chrono lancé tourne encore après un redémarrage.
  import { timerElapsed, timerReset, timerStart, timerStop } from "./ops";
  import { saveBlock } from "./store.svelte";
  import { formatClock, formatDuration, parseClock, type Block } from "./types";

  let { block }: { block: Block<"timer"> } = $props();

  let now = $state(Date.now());
  let record = $state(false);

  const d = $derived(block.data);
  const running = $derived(d.running_since != null);
  const elapsed = $derived(timerElapsed(d, now));
  const overTarget = $derived(d.target_ms != null && elapsed > d.target_ms);

  // Rafraîchit l'affichage seulement pendant la course
  $effect(() => {
    if (!running) return;
    const id = setInterval(() => (now = Date.now()), 47);
    return () => clearInterval(id);
  });

  function start() {
    now = Date.now();
    timerStart(block.data, now);
    record = false;
    saveBlock(block);
  }

  function stop() {
    now = Date.now();
    record = timerStop(block.data, now);
    saveBlock(block);
  }

  function reset() {
    timerReset(block.data);
    record = false;
    saveBlock(block);
  }

  function setTarget(raw: string) {
    const s = raw.trim() === "" ? null : parseClock(raw);
    block.data.target_ms = s == null ? null : Math.round(s * 1000);
    saveBlock(block);
  }
</script>

<div class="space-y-2">
  <input
    bind:value={block.data.label}
    oninput={() => saveBlock(block)}
    placeholder="Ce que vous chronométrez (ex. niveau 3 en moins de 5 min)"
    aria-label="Nom du chronomètre"
    class="w-full bg-transparent text-sm text-muted outline-none placeholder:text-muted/60 focus:text-fg"
  />

  <div class="flex items-center gap-3">
    <div
      class="min-w-0 flex-1 font-mono text-3xl font-bold tabular-nums
        {overTarget ? 'text-danger' : running ? 'text-fg' : elapsed > 0 && d.target_ms != null ? 'text-ok' : 'text-fg/90'}"
      aria-live="off"
    >
      {formatDuration(elapsed)}
    </div>
    {#if running}
      <button onclick={stop} class="rounded-lg bg-danger px-4 py-2 text-sm font-semibold text-white hover:brightness-110">Arrêter</button>
    {:else}
      <button onclick={start} class="rounded-lg bg-accent px-4 py-2 text-sm font-semibold text-on-accent hover:brightness-110">
        {elapsed > 0 ? "Recommencer" : "Démarrer"}
      </button>
    {/if}
    <button
      onclick={reset}
      disabled={elapsed === 0 && !running}
      class="rounded-lg border border-line px-3 py-2 text-sm text-muted hover:bg-raised disabled:opacity-40"
      title="Remettre à zéro">0</button
    >
  </div>

  <div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted">
    <label class="flex items-center gap-1">
      Objectif
      <input
        value={d.target_ms != null ? formatClock(d.target_ms / 1000) : ""}
        onchange={(e) => setTarget(e.currentTarget.value)}
        placeholder="aucun"
        aria-label="Temps cible (ex. 5:00)"
        class="w-14 rounded border border-line bg-bg px-1 text-center font-mono outline-none focus:border-accent"
      />
    </label>
    <span>
      Meilleur temps :
      <strong class="font-mono {record ? 'text-gold' : 'text-fg'}">{d.best_ms != null ? formatDuration(d.best_ms) : "—"}</strong>
      {#if record}<span class="text-gold">· nouveau record !</span>{/if}
    </span>
    {#if d.best_ms != null}
      <button
        onclick={() => {
          block.data.best_ms = null;
          record = false;
          saveBlock(block);
        }}
        class="underline hover:text-fg">effacer</button
      >
    {/if}
  </div>
</div>
