<script lang="ts">
  // Maquette : un bloc de guide, rendu sans enregistrement (repli, liste, compteur fonctionnent en mémoire).
  import type { Bloc } from "./donnees";

  let {
    bloc,
    first,
    last,
    onremove,
    onmove,
  }: { bloc: Bloc; first: boolean; last: boolean; onremove: () => void; onmove: (d: -1 | 1) => void } = $props();

  const INFO: Record<Bloc["kind"], { icon: string; label: string }> = {
    video: { icon: "▶", label: "Vidéo YouTube" },
    image: { icon: "▣", label: "Image" },
    text: { icon: "¶", label: "Texte" },
    list: { icon: "☑", label: "Liste" },
    counter: { icon: "#", label: "Compteur" },
    timer: { icon: "⏱", label: "Chronomètre" },
  };
  const info = $derived(INFO[bloc.kind]);

  function summary(): string {
    switch (bloc.kind) {
      case "text":
        return (bloc.text ?? "").split("\n").find((l) => l.trim()) ?? "";
      case "image":
        return bloc.caption ?? "";
      case "video":
        return bloc.title ?? "";
      case "list": {
        const it = bloc.items ?? [];
        return `${it.filter((i) => i.checked).length} / ${it.length}`;
      }
      case "counter":
        return bloc.goal != null ? `${bloc.value} / ${bloc.goal}` : String(bloc.value ?? 0);
      case "timer":
        return bloc.best ? `meilleur : ${bloc.best}` : "";
    }
  }
</script>

<section class="group/block rounded-lg border border-line bg-bg/60">
  <header class="flex items-center gap-1.5 px-2 pt-1.5 text-[11px] text-muted {bloc.collapsed ? 'pb-1.5' : ''}">
    <button
      onclick={() => (bloc.collapsed = !bloc.collapsed)}
      title={bloc.collapsed ? "Déplier" : "Replier"}
      class="grid h-5 w-5 shrink-0 place-items-center rounded hover:bg-raised hover:text-fg"
    >
      <span class="transition-transform {bloc.collapsed ? '-rotate-90' : ''}">▾</span>
    </button>
    <span aria-hidden="true">{info.icon}</span>
    {#if bloc.kind === "counter" || bloc.kind === "timer"}
      <input
        bind:value={bloc.label}
        placeholder={info.label}
        class="min-w-0 flex-1 rounded border border-transparent bg-transparent px-1 font-medium text-fg outline-none
          placeholder:tracking-wide placeholder:text-muted placeholder:uppercase hover:border-line focus:border-accent"
      />
    {:else}
      <span class="shrink-0 font-medium tracking-wide uppercase">{info.label}</span>
    {/if}
    {#if bloc.collapsed && summary()}<span class="min-w-0 truncate">· {summary()}</span>{/if}
    <span class="ml-auto flex shrink-0 gap-0.5 opacity-0 transition group-hover/block:opacity-100">
      <button onclick={() => onmove(-1)} disabled={first} class="rounded px-1.5 py-0.5 hover:bg-raised hover:text-fg disabled:opacity-30" title="Monter">↑</button>
      <button onclick={() => onmove(1)} disabled={last} class="rounded px-1.5 py-0.5 hover:bg-raised hover:text-fg disabled:opacity-30" title="Descendre">↓</button>
      <button onclick={onremove} class="rounded px-1.5 py-0.5 hover:bg-raised hover:text-danger" title="Supprimer">🗑</button>
    </span>
  </header>

  {#if !bloc.collapsed}
    <div class="px-2.5 pt-1 pb-2.5 text-sm">
      {#if bloc.kind === "text"}
        {#if bloc.text}
          {#each bloc.text.split("\n").filter((l) => l.trim()) as line, i (i)}
            <p class="leading-relaxed {i ? 'mt-1.5' : ''}">{line}</p>
          {/each}
        {:else}
          <p class="text-muted/70">Vos notes…</p>
        {/if}
      {:else if bloc.kind === "image"}
        <div
          class="grid aspect-video place-items-center rounded-md text-xs text-white/70"
          style="background: linear-gradient(135deg, #3b2f52, #1d2a3a 55%, #3a2a22)"
        >
          capture d'écran
        </div>
        {#if bloc.caption}<p class="mt-1 text-xs text-muted">{bloc.caption}</p>{/if}
      {:else if bloc.kind === "video"}
        <div class="relative grid aspect-video place-items-center rounded-md bg-black/70">
          <span class="grid h-10 w-14 place-items-center rounded-xl bg-[#e33] text-white">▶</span>
          {#if bloc.title}<span class="absolute top-2 left-2 right-2 truncate text-xs font-medium text-white/90">{bloc.title}</span>{/if}
        </div>
      {:else if bloc.kind === "list"}
        {@const items = bloc.items ?? []}
        <div class="mb-1 flex items-center gap-2 text-xs text-muted">
          {items.filter((i) => i.checked).length} / {items.length}
          <span class="h-1 flex-1 overflow-hidden rounded bg-line">
            <span class="block h-full bg-accent" style="width: {items.length ? (items.filter((i) => i.checked).length / items.length) * 100 : 0}%"></span>
          </span>
        </div>
        {#each items as it, i (i)}
          <label class="flex cursor-pointer items-center gap-2 py-0.5">
            <input type="checkbox" bind:checked={it.checked} class="accent-accent" />
            <span class={it.checked ? "text-muted line-through" : ""}>{it.text}</span>
          </label>
        {:else}
          <p class="text-muted/70">Liste vide</p>
        {/each}
      {:else if bloc.kind === "counter"}
        <div class="flex items-center gap-3">
          <button onclick={() => (bloc.value = Math.max(0, (bloc.value ?? 0) - 1))} class="h-9 w-9 rounded-lg border border-line text-lg hover:bg-raised">−</button>
          <div class="flex-1 text-center tabular-nums">
            <span class="text-2xl font-bold {bloc.goal != null && (bloc.value ?? 0) >= bloc.goal ? 'text-ok' : ''}">{bloc.value ?? 0}</span>
            <span class="text-lg text-muted"> / {bloc.goal ?? "∞"}</span>
            {#if bloc.goal}
              <div class="mx-auto mt-1 h-1.5 overflow-hidden rounded-full bg-line">
                <div class="h-full rounded-full bg-accent" style="width: {Math.min(100, ((bloc.value ?? 0) / bloc.goal) * 100)}%"></div>
              </div>
            {/if}
          </div>
          <button onclick={() => (bloc.value = (bloc.value ?? 0) + 1)} class="h-9 w-9 rounded-lg bg-accent text-lg font-semibold text-on-accent">+</button>
        </div>
      {:else if bloc.kind === "timer"}
        <div class="flex items-center gap-3">
          <span class="flex-1 font-mono text-2xl font-bold tabular-nums">0:00.00</span>
          <button class="rounded-lg bg-accent px-3 py-1.5 text-sm font-semibold text-on-accent">Démarrer</button>
        </div>
        <p class="mt-1 text-xs text-muted">Objectif 6:00:00 · Meilleur temps : {bloc.best ?? "—"}</p>
      {/if}
    </div>
  {/if}
</section>
