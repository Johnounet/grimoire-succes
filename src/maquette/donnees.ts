// Données factices de la maquette « vue unique des succès » (développement seulement, ?maquette).
// Jeu inventé : rien ne vient de Steam ni de la base.

export type BlocKind = "text" | "image" | "video" | "list" | "counter" | "timer";

export interface Bloc {
  id: number;
  kind: BlocKind;
  collapsed?: boolean;
  text?: string;
  caption?: string;
  title?: string;
  items?: { text: string; checked: boolean }[];
  label?: string;
  value?: number;
  goal?: number | null;
  best?: string | null;
}

export interface Succes {
  id: number;
  /** Rang dans l'ordre Steam. */
  steam: number;
  name: string;
  description: string;
  rarity: number;
  unlocked: boolean;
  section: number | null;
  tags: string[];
  blocks: Bloc[];
}

export interface Section {
  id: number;
  title: string;
  color: string;
  collapsed: boolean;
}

export const TAGS: Record<string, string> = {
  Ratable: "#ef5b5b",
  Immanquable: "#4cc38a",
  "Fin de partie": "#b07cff",
  Grind: "#e0a43a",
};

export const sections: Section[] = [
  { id: 1, title: "Histoire, chapitres 1 à 6", color: "#8b6cff", collapsed: false },
  { id: 2, title: "Collectibles", color: "#e0a43a", collapsed: false },
  { id: 3, title: "Difficulté et fin de partie", color: "#ef7a5b", collapsed: false },
];

let nextBloc = 1;
const b = (kind: BlocKind, data: Omit<Bloc, "id" | "kind"> = {}): Bloc => ({ id: nextBloc++, kind, ...data });
export const newBloc = (kind: BlocKind): Bloc =>
  b(kind, {
    text: "",
    caption: "",
    title: "",
    items: kind === "list" ? [] : undefined,
    label: "",
    value: 0,
    goal: kind === "counter" ? null : undefined,
    best: null,
  });

// [nom, description, rareté, obtenu, section, étiquettes, blocs]
type Row = [string, string, number, boolean, number | null, string[], Bloc[]?];

const rows: Row[] = [
  ["Premiers pas", "Terminer le prologue.", 92.4, true, 1, ["Immanquable"]],
  ["La porte du nord", "Terminer le chapitre 1.", 81.2, true, 1, ["Immanquable"]],
  ["Fidèle au poste", "Garder Aldric en vie jusqu'à la fin du chapitre 2.", 34.1, false, 1, ["Ratable"], [
    b("text", { text: "CHAPITRE 2 — Pendant l'assaut du pont, Aldric fonce seul vers la tour. Ne pas le laisser faire : parlez-lui AVANT de traverser (option « Reste avec moi »).\n\nS'il meurt, le succès est perdu pour la partie : sauvegardez avant le pont." }),
    b("image", { caption: "Le pont : l'option de dialogue apparaît près du brasero." }),
  ]],
  ["Le marché noir", "Terminer le chapitre 2.", 70.3, false, 1, ["Immanquable"]],
  ["Sans un bruit", "Traverser le camp de Brume sans être repéré.", 12.6, false, 1, ["Ratable"], [
    b("video", { title: "Camp de Brume — infiltration complète sans alerte" }),
    b("list", { items: [
      { text: "Éteindre la torche de l'entrée", checked: true },
      { text: "Attendre la relève des gardes", checked: true },
      { text: "Passer par la palissade est", checked: false },
      { text: "Ne pas ramasser le coffre du chef", checked: false },
    ] }),
  ]],
  ["Les cendres d'Orvel", "Terminer le chapitre 3.", 61.7, false, 1, ["Immanquable"]],
  ["Diplomate", "Convaincre les trois clans sans combattre.", 8.9, false, 1, ["Ratable"], [
    b("text", { text: "Réponses à choisir :\n- Clan du Loup : « Vos terres resteront les vôtres. »\n- Clan du Corbeau : montrer la lettre du chapitre 2.\n- Clan de l'Ours : gagner le bras de fer (QTE)." }),
  ]],
  ["Le roi déchu", "Terminer le chapitre 4.", 54.0, false, 1, ["Immanquable"]],
  ["Un dernier verre", "Partager un verre avec chaque compagnon avant le chapitre 5.", 15.3, false, 1, ["Ratable"], [
    b("list", { items: [
      { text: "Aldric", checked: false },
      { text: "Mira", checked: false },
      { text: "Le Borgne", checked: false },
      { text: "Sœur Ilse", checked: false },
    ] }),
  ]],
  ["Au bout de la nuit", "Terminer le chapitre 5.", 47.8, false, 1, ["Immanquable"]],
  ["Valmont", "Terminer l'histoire.", 41.2, false, 1, ["Immanquable"]],
  ["Plumes d'argent", "Trouver les 30 plumes d'argent.", 6.4, false, 2, ["Grind"], [
    b("counter", { label: "Plumes trouvées", value: 17, goal: 30 }),
    b("image", { caption: "Carte des plumes (zones 1 à 4)." }),
    b("text", { text: "Les plumes 21 à 30 ne sont accessibles qu'après le chapitre 4 (grappin)." }),
  ]],
  ["Bibliophile", "Lire tous les livres de la bibliothèque d'Orvel.", 11.0, false, 2, [], [
    b("counter", { label: "Livres lus", value: 4, goal: 12 }),
  ]],
  ["Cartographe", "Révéler toute la carte du monde.", 19.5, false, 2, []],
  ["Trésors engloutis", "Ouvrir les 8 coffres sous-marins.", 9.7, false, 2, [], [
    b("list", { items: [
      { text: "Lac d'Orvel — sous la cascade", checked: true },
      { text: "Port de Brume — épave", checked: true },
      { text: "Grotte aux méduses", checked: true },
      { text: "Baie du Roi (2 coffres)", checked: false },
      { text: "Puits du monastère", checked: false },
      { text: "Rivière noire", checked: false },
      { text: "Fosse de Valmont", checked: false },
    ] }),
  ]],
  ["Collectionneur", "Obtenir toutes les cartes du jeu de Tarot.", 4.1, false, 2, ["Grind", "Ratable"], [
    b("text", { text: "3 cartes sont ratables : le Pendu (chapitre 2, marchand ambulant), la Lune (chapitre 3, avant l'incendie) et la Tour (chapitre 5)." }),
  ]],
  ["Chasseur de primes", "Remplir les 20 contrats de prime.", 13.8, false, 2, ["Grind"]],
  ["Herboriste", "Cueillir chaque plante au moins une fois.", 22.6, true, 2, []],
  ["Dur à cuire", "Terminer le jeu en difficulté Légende.", 2.3, false, 3, ["Fin de partie"], [
    b("text", { text: "Faire une partie Nouvelle partie + : la difficulté Légende garde l'équipement. Prioriser l'armure de Mira (chapitre 4)." }),
    b("video", { title: "Boss final en Légende — stratégie sans dégâts" }),
  ]],
  ["Contre la montre", "Terminer le jeu en moins de 6 heures.", 1.8, false, 3, ["Fin de partie"], [
    b("timer", { label: "Run complet", best: null }),
  ]],
  ["Intouchable", "Vaincre le Roi déchu sans subir de dégâts.", 5.2, false, 3, [], [
    b("video", { title: "Le Roi déchu sans dégâts — patterns expliqués" }),
  ]],
  ["Pacifiste", "Terminer le chapitre 3 sans tuer personne.", 7.4, false, 3, ["Ratable"]],
  ["Mains nues", "Vaincre 50 ennemis sans arme.", 18.2, false, null, ["Grind"], [
    b("counter", { label: "", value: 32, goal: 50 }),
  ]],
  ["Pêcheur du dimanche", "Attraper un poisson.", 66.0, true, null, []],
  ["Le gros lot", "Attraper la carpe dorée.", 3.6, false, null, [], [
    b("text", { text: "Lac d'Orvel, la nuit, sous la pluie, avec l'appât « ver luisant ». Environ 1 chance sur 20." }),
  ]],
  ["Joueur de dés", "Gagner 10 parties de dés.", 24.9, false, null, []],
  ["Riche à millions", "Posséder 100 000 écus.", 9.1, false, null, ["Grind"]],
  ["Cuisinier", "Préparer chaque recette.", 14.4, false, null, []],
  ["Ami des bêtes", "Caresser chaque animal.", 37.5, true, null, []],
  ["Forgeron", "Améliorer une arme au niveau maximal.", 28.0, false, null, []],
  ["Pas de quartier", "Vaincre 1 000 ennemis.", 31.3, false, null, ["Grind"]],
  ["Touriste", "Visiter chaque auberge.", 26.1, false, null, []],
  ["Musicien", "Jouer les 5 airs du luth.", 10.2, false, null, []],
  ["Vieux loup de mer", "Naviguer jusqu'à l'île oubliée.", 20.7, false, null, []],
  ["Monte-en-l'air", "Crocheter 25 serrures.", 33.0, true, null, []],
  ["Insomniaque", "Ne pas dormir pendant 10 jours de jeu.", 6.9, false, null, []],
  ["Champion de l'arène", "Remporter le tournoi d'Orvel.", 16.4, false, null, []],
  ["Bavard", "Épuiser tous les dialogues de Mira.", 12.0, false, null, ["Ratable"]],
  ["Taxidermiste", "Vaincre les 6 bêtes légendaires.", 8.3, false, null, []],
  ["Sur le fil", "Gagner un combat avec 1 PV.", 21.8, false, null, []],
];

export const succes: Succes[] = rows.map(([name, description, rarity, unlocked, section, tags, blocks], i) => ({
  id: 100 + i,
  steam: i + 1,
  name,
  description,
  rarity,
  unlocked,
  section,
  tags,
  blocks: blocks ?? [],
}));
