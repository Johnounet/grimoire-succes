<script lang="ts">
  // Bloc-notes personnel du jeu (onglet « Bloc-notes », demande de l'utilisateur, 2026-11-15) : un grand
  // texte libre, à part du guide (jamais exporté). Même éditeur que le bloc « Texte ».
  import { api } from "./library";
  import TextBlock from "./blocks/TextBlock.svelte";
  import type { Block } from "./blocks/types";
  import { deferSave, flushSave } from "./blocks/store.svelte";

  let { appid }: { appid: number } = $props();

  /** Faux bloc « Texte » : TextBlock l'édite, `save` l'enregistre dans `game_notes`. */
  let note = $state<Block<"text"> | null>(null);
  let error = $state("");

  $effect(() => {
    const id = appid;
    api
      .gameNotesGet(id)
      .then((body) => {
        note = { id: 0, kind: "text", position: 0, pinned: false, achievement_id: null, card_id: null, data: { markdown: body } } as Block<"text">;
      })
      .catch((e) => (error = String(e)));
    // Changement de jeu ou fermeture de l'onglet : ce qui reste à enregistrer part tout de suite
    return () => flushSave(`notes:${id}`);
  });

  /** Enregistrement différé, comme les blocs (et envoyé avec eux à la fermeture de la fenêtre). */
  function save() {
    const id = appid;
    if (!note) return;
    // Copie simple du texte, prise ici : lu dans le nettoyage du composant (changement de jeu), un `$state`
    // rend son ancienne valeur, et la dernière frappe serait perdue (vu à l'essai).
    const body = note.data.markdown;
    deferSave(`notes:${id}`, () =>
      api.gameNotesSet(id, body).then(
        () => void (error = ""),
        (e) => void (error = `Bloc-notes non enregistré : ${e}`),
      ),
    );
  }
</script>

<div class="mx-auto w-full max-w-3xl px-6 pt-4 pb-10">
  <p class="mb-3 text-xs text-muted">
    Vos notes personnelles sur ce jeu, enregistrées au fil de la frappe. Elles restent ici : l'export de guide ne les emporte pas.
  </p>
  {#if error}<p class="mb-3 rounded-lg bg-danger/10 p-3 text-sm text-danger">{error}</p>{/if}
  {#if note}
    <div class="notes min-h-[50vh] rounded-lg border border-line bg-panel p-4">
      <TextBlock
        block={note}
        onsave={save}
        rows={18}
        placeholder={"Écrivez ici tout ce que vous voulez : objectifs, choses à ne pas oublier, où vous en êtes…\n\n(**gras**, ## titre, - liste, [lien](https://…))"}
      />
    </div>
  {/if}
</div>

<style>
  /* Tout le cadre est cliquable pour écrire, pas seulement les lignes déjà tapées */
  .notes :global([role="article"]) {
    min-height: calc(50vh - 2rem);
  }
</style>
