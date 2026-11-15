<script lang="ts">
  // MAQUETTE (développement seulement, http://localhost:1420/?maquette) : vue unique des succès.
  // Liste et espace de travail fusionnés : tous les succès du jeu, en colonnes automatiques,
  // rangés par sections, avec tri, filtres et déplacement par clics. Rien n'est enregistré.
  import Carte from "./Carte.svelte";
  import { sections as baseSections, succes as baseSucces, TAGS, type Succes } from "./donnees";

  const beaucoup = new URLSearchParams(location.search).has("beaucoup");
  // « ?maquette&beaucoup » : 200 succès, pour juger la lisibilité d'un gros jeu
  const extra: Succes[] = beaucoup
    ? Array.from({ length: 160 }, (_, i) => ({
        id: 1000 + i,
        steam: 41 + i,
        name: `Succès de remplissage n° ${i + 1}`,
        description: "Description d'un succès sans guide, pour voir l'effet d'un grand nombre de succès.",
        rarity: Math.round(((i * 37) % 900) / 10 + 1) / 1,
        unlocked: i % 3 === 0,
        section: null,
        tags: [],
        blocks: [],
      }))
    : [];

  let items = $state<Succes[]>([...baseSucces, ...extra]);
  let sections = $state(baseSections);

  // ----- Filtres et tri -----
  let etat = $state<"todo" | "done" | "all">("todo");
  let sort = $state<"mine" | "steam" | "rarity">("mine");
  let query = $state("");
  let tag = $state("");
  let guidedOnly = $state(false);

  const match = (s: Succes) =>
    (etat === "all" || (etat === "done") === s.unlocked) &&
    (!guidedOnly || s.blocks.length > 0) &&
    (!tag || s.tags.includes(tag)) &&
    (!query.trim() || `${s.name} ${s.description}`.toLowerCase().includes(query.trim().toLowerCase()));

  function sorted(list: Succes[]): Succes[] {
    if (sort === "steam") return [...list].sort((a, b) => a.steam - b.steam);
    if (sort === "rarity") return [...list].sort((a, b) => a.rarity - b.rarity);
    return list;
  }

  const groups = $derived(
    [...sections.map((s) => ({ id: s.id as number | null, title: s.title, color: s.color, sec: s })), { id: null, title: "Autres succès", color: "#7d8597", sec: null }].map((g) => {
      const all = items.filter((s) => s.section === g.id);
      return { ...g, all, list: sorted(all.filter(match)) };
    }),
  );
  const shown = $derived(groups.reduce((n, g) => n + g.list.length, 0));
  const unlocked = $derived(items.filter((s) => s.unlocked).length);

  // ----- Colonnes automatiques (chaque carte va dans la colonne la plus courte) -----
  const MIN_COL = 340;
  const GAP = 10;
  let width = $state(0);
  const cols = $derived(Math.max(1, Math.floor((width + GAP) / (MIN_COL + GAP))));
  let heights = $state<Record<number, number>>({});
  const estimate = (s: Succes) => (s.blocks.length ? 160 + 120 * s.blocks.length : 52);

  /** « suite » : l'ordre se lit colonne par colonne (comme un journal) ; « court » : colonne la plus courte. */
  let packing = $state<"suite" | "court">("suite");

  function columns(list: Succes[]): Succes[][] {
    if (packing === "suite") return inSequence(list);
    const h = Array<number>(cols).fill(0);
    const out: Succes[][] = Array.from({ length: cols }, () => []);
    for (const s of list) {
      const i = h.indexOf(Math.min(...h));
      out[i].push(s);
      h[i] += (heights[s.id] ?? estimate(s)) + GAP;
    }
    return out;
  }

  /** Colonnes remplies l'une après l'autre, de hauteurs aussi proches que possible. */
  function inSequence(list: Succes[]): Succes[][] {
    const size = (s: Succes) => (heights[s.id] ?? estimate(s)) + GAP;
    const total = list.reduce((n, s) => n + size(s), 0);
    const out: Succes[][] = Array.from({ length: cols }, () => []);
    let c = 0;
    let acc = 0;
    for (const s of list) {
      const h = size(s);
      // Passe à la colonne suivante quand celle-ci dépasserait sa part (sauf la dernière)
      if (c < cols - 1 && out[c].length && acc + h / 2 > (total * (c + 1)) / cols) c++;
      out[c].push(s);
      acc += h;
    }
    return out;
  }

  // ----- Déplacement par clics -----
  let moving = $state<number | null>(null);
  let swap = $state(false);
  let flash = $state<number | null>(null);
  const movingItem = $derived(items.find((s) => s.id === moving));

  function done(id: number) {
    moving = null;
    flash = id;
    setTimeout(() => flash === id && (flash = null), 1300);
  }

  function pick(target: Succes) {
    if (moving == null) return;
    const from = items.findIndex((s) => s.id === moving);
    const m = items[from];
    if (swap) {
      const to = items.findIndex((s) => s.id === target.id);
      const sec = m.section;
      m.section = target.section;
      target.section = sec;
      items[from] = target;
      items[to] = m;
    } else {
      items.splice(from, 1);
      m.section = target.section;
      items.splice(items.findIndex((s) => s.id === target.id), 0, m);
    }
    done(m.id);
  }

  /** Clic sur le titre d'une section pendant un déplacement : à la fin de cette section. */
  function toSectionEnd(id: number | null) {
    if (moving == null) return;
    const from = items.findIndex((s) => s.id === moving);
    const [m] = items.splice(from, 1);
    m.section = id;
    let last = -1;
    items.forEach((s, i) => s.section === id && (last = i));
    items.splice(last + 1, 0, m);
    done(m.id);
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && (moving = null)} />

<div class="flex h-screen text-fg">
  <!-- Panneau des jeux (décor) -->
  <aside class="flex w-56 shrink-0 flex-col gap-1 border-r border-line bg-panel p-2 text-sm">
    <div class="mb-2 rounded-lg bg-accent px-3 py-2 text-center font-medium text-on-accent">+ Ajouter un jeu</div>
    <div class="rounded-md px-2 py-2 text-muted">🏠 Accueil</div>
    <div class="rounded-md bg-raised px-2 py-2"><div class="font-medium">Chroniques de Valmont</div><div class="text-xs text-muted">{unlocked} / {items.length}</div></div>
    <div class="rounded-md px-2 py-2 text-muted">Hades<div class="text-xs">31 / 49</div></div>
    <div class="mt-auto rounded-md border border-dashed border-gold/60 p-2 text-[11px] text-gold">
      Maquette : rien n'est enregistré. Rechargez la page pour revenir au départ.
      {#if !beaucoup}<a href="?maquette&beaucoup" class="mt-1 block underline">Voir avec 200 succès</a>{:else}<a href="?maquette" class="mt-1 block underline">Revenir à 40 succès</a>{/if}
    </div>
  </aside>

  <main class="flex min-w-0 flex-1 flex-col">
    <!-- Bannière du jeu (décor) -->
    <div class="shrink-0 border-b border-line px-6 py-4" style="background: linear-gradient(100deg, #2a1d3d, #121722 60%)">
      <div class="flex items-end gap-4">
        <div class="min-w-0 flex-1">
          <h1 class="text-2xl font-bold">Chroniques de Valmont</h1>
          <p class="text-sm text-muted">{unlocked} / {items.length} succès · {Math.round((unlocked / items.length) * 100)} %</p>
        </div>
        <span class="rounded-md border border-line px-3 py-1 text-sm">Mode compact</span>
        <span class="rounded-md border border-line px-3 py-1 text-sm">Actualiser</span>
        <span class="rounded-md border border-line px-3 py-1 text-sm">⋯</span>
      </div>
    </div>

    <!-- Barre d'outils : ce qui était le panneau de droite -->
    <div class="flex shrink-0 flex-wrap items-center gap-2 border-b border-line bg-panel/60 px-6 py-2 text-sm">
      <input
        bind:value={query}
        placeholder="⌕ Rechercher un succès…"
        class="w-56 rounded-md border border-line bg-bg px-2.5 py-1 outline-none focus:border-accent"
      />
      <div class="inline-flex rounded-lg border border-line bg-bg p-0.5">
        {#each [["todo", "À obtenir"], ["done", "Obtenus"], ["all", "Tous"]] as [v, l] (v)}
          <button
            onclick={() => (etat = v as typeof etat)}
            class="rounded-md px-2.5 py-0.5 {etat === v ? 'bg-raised font-medium' : 'text-muted hover:text-fg'}">{l}</button
          >
        {/each}
      </div>
      <select bind:value={sort} class="rounded-md border border-line bg-bg px-2 py-1">
        <option value="mine">Mon ordre</option>
        <option value="steam">Ordre Steam</option>
        <option value="rarity">Les plus rares d'abord</option>
      </select>
      <select bind:value={tag} class="rounded-md border border-line bg-bg px-2 py-1">
        <option value="">Toutes les étiquettes</option>
        {#each Object.keys(TAGS) as t (t)}<option value={t}>{t}</option>{/each}
      </select>
      <select bind:value={packing} class="rounded-md border border-line bg-bg px-2 py-1" title="Comment les cartes se répartissent dans les colonnes">
        <option value="suite">Rangement : à la suite (colonne par colonne)</option>
        <option value="court">Rangement : colonne la plus courte</option>
      </select>
      <label class="flex items-center gap-1.5 text-muted"><input type="checkbox" bind:checked={guidedOnly} class="accent-accent" /> Avec guide seulement</label>
      <span class="ml-auto text-xs text-muted">{shown} affichés</span>
      <span class="rounded-md border border-line px-2 py-0.5 text-xs text-muted">+ Section</span>
      <span class="rounded-md border border-line px-2 py-0.5 text-xs text-muted">+ Carte libre</span>
    </div>

    {#if movingItem}
      <div class="flex shrink-0 items-center gap-3 border-b border-accent/50 bg-accent/15 px-6 py-2 text-sm">
        <span class="min-w-0 flex-1">
          Déplacement de <strong>« {movingItem.name} »</strong> : cliquez sur un succès, ou sur le titre d'une section pour le mettre à la fin.
        </span>
        <div class="inline-flex rounded-lg border border-line bg-bg p-0.5 text-xs">
          <button onclick={() => (swap = false)} class="rounded-md px-2 py-0.5 {!swap ? 'bg-raised font-medium' : 'text-muted'}">Placer avant</button>
          <button onclick={() => (swap = true)} class="rounded-md px-2 py-0.5 {swap ? 'bg-raised font-medium' : 'text-muted'}">Échanger</button>
        </div>
        <button onclick={() => (moving = null)} class="rounded-md border border-line px-2 py-0.5 hover:bg-raised">Annuler (Échap)</button>
      </div>
    {/if}

    <div class="min-h-0 flex-1 overflow-y-auto px-6 py-4">
      <div bind:clientWidth={width} class="space-y-4">
        {#each groups as g (g.id ?? 0)}
          {@const collapsed = g.sec?.collapsed ?? false}
          <section class="zone rounded-md" style="--zone-color: {g.color}">
            <header class="zone-head flex items-center gap-2 rounded-t-[5px] border-b px-3 py-1.5 text-sm">
              <button
                onclick={() => g.sec && (g.sec.collapsed = !g.sec.collapsed)}
                class="grid h-6 w-6 place-items-center rounded hover:bg-raised {g.sec ? '' : 'invisible'}"
                title={collapsed ? "Déplier" : "Replier"}
              >
                <span class="transition-transform {collapsed ? '-rotate-90' : ''}">▾</span>
              </button>
              <span class="h-2.5 w-2.5 rounded-full" style="background: {g.color}"></span>
              <span class="font-semibold">{g.title}</span>
              <span class="text-xs text-muted">{g.all.filter((s) => s.unlocked).length} / {g.all.length}</span>
              <span class="h-1.5 w-20 overflow-hidden rounded-sm bg-raised">
                <span class="block h-full" style="width: {g.all.length ? (g.all.filter((s) => s.unlocked).length / g.all.length) * 100 : 0}%; background: {g.color}"></span>
              </span>
              {#if moving != null}
                <button onclick={() => toSectionEnd(g.id)} class="ml-auto rounded-md bg-accent px-2 py-0.5 text-xs font-semibold text-on-accent">
                  Mettre à la fin de cette section
                </button>
              {/if}
            </header>
            {#if !collapsed}
              <div class="p-2.5">
                {#if g.list.length}
                  <div class="flex items-start" style="gap: {GAP}px">
                    {#each columns(g.list) as col, ci (ci)}
                      <div class="flex min-w-0 flex-1 flex-col" style="gap: {GAP}px">
                        {#each col as s (s.id)}
                          <div bind:clientHeight={heights[s.id]} class="relative">
                            {#if moving != null && sort === "mine"}
                              <span class="absolute -top-2 -left-2 z-20 rounded-full bg-raised px-1.5 text-[10px] font-semibold text-muted ring-1 ring-line">{g.list.indexOf(s) + 1}</span>
                            {/if}
                            <Carte
                              {s}
                              {moving}
                              {swap}
                              canMove={sort === "mine"}
                              flash={flash === s.id}
                              onstartmove={() => (moving = s.id)}
                              onpick={() => pick(s)}
                            />
                          </div>
                        {/each}
                      </div>
                    {/each}
                  </div>
                {:else}
                  <p class="px-1 py-2 text-sm text-muted">Rien à afficher avec ces filtres.</p>
                {/if}
              </div>
            {/if}
          </section>
        {/each}
      </div>
    </div>
  </main>
</div>

<style>
  .zone {
    border: 1px solid color-mix(in srgb, var(--zone-color) 60%, transparent);
    background: color-mix(in srgb, var(--zone-color) 6%, var(--color-bg));
  }
  .zone-head {
    background: color-mix(in srgb, var(--zone-color) 22%, var(--color-panel));
    border-color: color-mix(in srgb, var(--zone-color) 50%, transparent);
  }
</style>
