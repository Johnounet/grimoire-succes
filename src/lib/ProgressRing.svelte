<script lang="ts">
  let { value, size = 28 }: { value: number; size?: number } = $props();

  const stroke = 3;
  const r = $derived((size - stroke) / 2);
  const c = $derived(2 * Math.PI * r);
  const done = $derived(value >= 100);
</script>

<svg width={size} height={size} viewBox="0 0 {size} {size}" class="shrink-0 -rotate-90" aria-hidden="true">
  <circle cx={size / 2} cy={size / 2} {r} fill="none" stroke="var(--color-line)" stroke-width={stroke} />
  <circle
    cx={size / 2}
    cy={size / 2}
    {r}
    fill="none"
    stroke={done ? "var(--color-gold)" : "var(--color-accent)"}
    stroke-width={stroke}
    stroke-linecap="round"
    stroke-dasharray={c}
    stroke-dashoffset={c * (1 - Math.min(value, 100) / 100)}
    class="transition-[stroke-dashoffset] duration-500"
  />
</svg>
