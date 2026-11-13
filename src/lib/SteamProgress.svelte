<script lang="ts">
  // Progression d'un succès suivie par Steam (« 37 / 50 »), lue dans le cache local et mise à jour
  // par l'actualisation automatique pendant que le joueur joue. Lecture seule.
  import type { Progress } from "./library";

  let { progress, small = false }: { progress: Progress; small?: boolean } = $props();

  const ratio = $derived(Math.min(1, Math.max(0, (progress.value - progress.min) / (progress.max - progress.min))));
  // Grandes valeurs (distances, argent) arrondies ; petites valeurs à virgule gardées à 1 décimale
  const fmt = (v: number) =>
    v.toLocaleString("fr-FR", { maximumFractionDigits: Math.abs(v) >= 100 || Number.isInteger(v) ? 0 : 1 });
  const text = $derived(`${fmt(progress.value)} / ${fmt(progress.max)}`);
</script>

<div
  class="flex items-center gap-2 {small ? 'text-[10px]' : 'text-xs'}"
  title="Progression suivie par Steam"
  role="progressbar"
  aria-label="Progression Steam"
  aria-valuemin={progress.min}
  aria-valuemax={progress.max}
  aria-valuenow={progress.value}
  aria-valuetext={text}
>
  <div class="h-1.5 min-w-0 flex-1 overflow-hidden rounded-full bg-line">
    <div class="h-full rounded-full bg-accent transition-[width]" style:width="{ratio * 100}%"></div>
  </div>
  <span class="shrink-0 font-medium tabular-nums text-muted">{text}</span>
</div>
