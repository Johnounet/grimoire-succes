<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { tick } from "svelte";
  import { renderMarkdown, TEXT_COLORS } from "../markdown";
  import { saveBlock } from "./store.svelte";
  import type { Block } from "./types";

  let { block }: { block: Block<"text"> } = $props();

  // Valeur initiale seulement, voulu : un bloc vide s'ouvre en édition
  // svelte-ignore state_referenced_locally
  let editing = $state(block.data.markdown === "");
  let area = $state<HTMLTextAreaElement>();
  /** Nuancier des couleurs du texte ouvert dans la barre. */
  let palette = $state(false);

  const html = $derived(renderMarkdown(block.data.markdown));

  async function edit() {
    editing = true;
    await tick();
    area?.focus();
    autosize();
  }

  function autosize() {
    if (!area) return;
    area.style.height = "auto";
    area.style.height = `${area.scrollHeight + 2}px`;
  }

  function input() {
    autosize();
    saveBlock(block);
  }

  /** Entoure la sélection (ou insère en début de ligne) puis enregistre. */
  function wrap(before: string, after = before, linePrefix = false) {
    if (!area) return;
    const { selectionStart: s, selectionEnd: e, value } = area;
    let next: string;
    let caret: number;
    if (linePrefix) {
      const lineStart = value.lastIndexOf("\n", s - 1) + 1;
      next = value.slice(0, lineStart) + before + value.slice(lineStart);
      caret = e + before.length;
    } else {
      const sel = value.slice(s, e) || (before === "[" ? "texte" : "");
      next = value.slice(0, s) + before + sel + after + value.slice(e);
      caret = s + before.length + sel.length;
    }
    block.data.markdown = next;
    saveBlock(block);
    tick().then(() => {
      area?.focus();
      area?.setSelectionRange(caret, caret);
      autosize();
    });
  }

  function keydown(e: KeyboardEvent) {
    if (!(e.ctrlKey || e.metaKey)) return;
    if (e.key === "b") (e.preventDefault(), wrap("**"));
    else if (e.key === "i") (e.preventDefault(), wrap("*"));
    else if (e.key === "k") (e.preventDefault(), wrap("[", "](https://)"));
  }

  /** Les liens s'ouvrent dans le navigateur du système. */
  function click(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest("a[data-external]") as HTMLAnchorElement | null;
    if (a) {
      e.preventDefault();
      e.stopPropagation();
      openUrl(a.href).catch(() => {});
      return;
    }
    edit();
  }

  const tools = [
    { label: "G", title: "Gras (Ctrl+B)", cls: "font-bold", run: () => wrap("**") },
    { label: "I", title: "Italique (Ctrl+I)", cls: "italic", run: () => wrap("*") },
    { label: "T", title: "Titre", cls: "font-semibold", run: () => wrap("## ", "", true) },
    { label: "•", title: "Liste à puces", cls: "", run: () => wrap("- ", "", true) },
    { label: "1.", title: "Liste numérotée", cls: "", run: () => wrap("1. ", "", true) },
    { label: "🔗", title: "Lien (Ctrl+K)", cls: "", run: () => wrap("[", "](https://)") },
    { label: "⚠", title: "Encadré d'avertissement (ligne commençant par « !! »)", cls: "", run: () => wrap("!! ", "", true) },
  ];

  function color(name: string) {
    palette = false;
    wrap(`{${name}}`, "{/}");
  }
</script>

{#if editing}
  <div class="space-y-1.5">
    <div class="flex flex-wrap gap-1" role="toolbar" aria-label="Mise en forme">
      {#each tools as t (t.title)}
        <button
          onmousedown={(e) => e.preventDefault()}
          onclick={t.run}
          title={t.title}
          aria-label={t.title}
          class="h-7 min-w-7 rounded border border-line px-1.5 text-xs text-muted hover:bg-raised hover:text-fg {t.cls}"
        >
          {t.label}
        </button>
      {/each}
      <!-- Couleur du texte sélectionné : palette fixe -->
      <button
        onmousedown={(e) => e.preventDefault()}
        onclick={() => (palette = !palette)}
        aria-expanded={palette}
        title="Couleur du texte sélectionné"
        aria-label="Couleur du texte sélectionné"
        class="h-7 min-w-7 rounded border border-line px-1.5 text-xs font-bold text-muted hover:bg-raised hover:text-fg"
      >
        <span class="border-b-2 border-danger">A</span>
      </button>
      {#if palette}
        {#each TEXT_COLORS as c (c.name)}
          <button
            onmousedown={(e) => e.preventDefault()}
            onclick={() => color(c.name)}
            title={c.label}
            aria-label={c.label}
            class="tc-{c.name} grid h-7 w-7 place-items-center rounded border border-line text-xs font-bold hover:bg-raised"
          >
            A
          </button>
        {/each}
      {/if}
      <button onclick={() => (editing = false)} class="ml-auto rounded px-2 text-xs text-muted hover:text-fg">Terminé</button>
    </div>
    <textarea
      bind:this={area}
      bind:value={block.data.markdown}
      oninput={input}
      onkeydown={keydown}
      onblur={() => {
        if (block.data.markdown.trim() !== "") editing = false;
      }}
      rows="3"
      placeholder={"Vos notes… (**gras**, ## titre, - liste, [lien](https://…), {rouge}texte{/}, !! avertissement)"}
      class="block w-full resize-none rounded-lg border border-line bg-bg px-3 py-2 text-sm leading-relaxed outline-none focus:border-accent"
    ></textarea>
  </div>
{:else}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    role="article"
    onclick={click}
    onkeydown={(e) => e.key === "Enter" && edit()}
    title="Cliquer pour modifier"
    class="md cursor-text rounded-lg px-1 py-0.5 text-sm leading-relaxed hover:bg-raised/40"
  >
    {@html html}
  </div>
{/if}

<style>
  .md :global(h3) {
    font-size: 1.05rem;
    font-weight: 700;
    margin: 0.6em 0 0.3em;
  }
  .md :global(h4) {
    font-size: 0.95rem;
    font-weight: 700;
    margin: 0.6em 0 0.3em;
  }
  .md :global(h5) {
    font-size: 0.85rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    margin: 0.6em 0 0.3em;
    color: var(--color-muted);
  }
  .md :global(p) {
    margin: 0.35em 0;
  }
  .md :global(ul) {
    list-style: disc;
    padding-left: 1.3em;
    margin: 0.35em 0;
  }
  .md :global(ol) {
    list-style: decimal;
    padding-left: 1.5em;
    margin: 0.35em 0;
  }
  .md :global(a) {
    color: var(--color-accent);
    text-decoration: underline;
  }
  .md :global(strong) {
    font-weight: 700;
  }
  /* Couleurs du texte : celles du thème, lisibles sur Nuit comme sur Papier */
  .md :global(.tc-rouge),
  .tc-rouge {
    color: var(--color-danger);
  }
  .md :global(.tc-orange),
  .tc-orange {
    color: color-mix(in srgb, var(--color-danger) 45%, var(--color-gold));
  }
  .md :global(.tc-vert),
  .tc-vert {
    color: var(--color-ok);
  }
  .md :global(.tc-or),
  .tc-or {
    color: var(--color-gold);
  }
  /* Encadré d'avertissement (lignes « !! ») */
  .md :global(.callout) {
    position: relative;
    margin: 0.5em 0;
    padding: 0.4em 0.7em 0.4em 2em;
    border-left: 3px solid var(--color-danger);
    border-radius: 4px;
    background: color-mix(in srgb, var(--color-danger) 13%, transparent);
    font-weight: 500;
  }
  .md :global(.callout)::before {
    content: "⚠";
    position: absolute;
    left: 0.6em;
    top: 0.4em;
    color: var(--color-danger);
  }
</style>
