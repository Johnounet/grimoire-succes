// Mini rendu Markdown pour le bloc Texte : titres, gras, italique, listes, liens, paragraphes,
// texte en couleur (`{rouge}texte{/}`, palette fixe `TEXT_COLORS`) et encadrés d'avertissement
// (lignes qui commencent par `!! `).
// Le texte est d'abord échappé : aucune balise HTML écrite par l'utilisateur ne passe.
// Volontairement minimal : pas de dépendance, et rien d'exécutable dans le résultat.

const escape = (s: string) =>
  s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#39;");

/** Couleurs du texte : une palette fixe, réglée pour chaque thème (CSS du bloc Texte). */
export const TEXT_COLORS = [
  { name: "rouge", label: "Rouge : danger, ratable" },
  { name: "orange", label: "Orange : attention" },
  { name: "vert", label: "Vert : astuce" },
  { name: "or", label: "Or : important" },
] as const;
/** `{rouge}texte{/}` : mêmes noms que `TEXT_COLORS`. */
const COLOR_RE = /\{(rouge|orange|vert|or)\}(.+?)\{\/\}/g;

/** Texte sans ses marques de couleur ni d'encadré (résumé d'un bloc replié). */
export const plainText = (s: string) => s.replace(COLOR_RE, "$2").replace(/^!!\s+/, "");

/** Seuls les liens http(s) sont rendus cliquables. */
const safeUrl = (u: string) => (/^https?:\/\//i.test(u) ? u : null);

function inline(text: string): string {
  let s = escape(text);
  // [texte](url)
  s = s.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (m, label: string, url: string) => {
    const href = safeUrl(url.replace(/&amp;/g, "&"));
    return href ? `<a href="${escape(href)}" data-external>${label}</a>` : m;
  });
  // URL nue
  s = s.replace(/(^|[\s(])(https?:\/\/[^\s<)]+)/g, (_m, pre: string, url: string) => {
    const href = url.replace(/&amp;/g, "&");
    return `${pre}<a href="${escape(href)}" data-external>${url}</a>`;
  });
  // {rouge}texte{/} : seules les couleurs de la palette passent (classe CSS, aucun style libre)
  s = s.replace(COLOR_RE, '<span class="tc-$1">$2</span>');
  s = s.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>");
  s = s.replace(/(^|[^*])\*(?!\s)(.+?)\*(?!\*)/g, "$1<em>$2</em>");
  return s;
}

export function renderMarkdown(md: string): string {
  const out: string[] = [];
  let list: "ul" | "ol" | null = null;
  let para: string[] = [];
  /** Lignes d'un encadré d'avertissement en cours (lignes consécutives en « !! »). */
  let callout: string[] = [];

  const closePara = () => {
    if (para.length) out.push(`<p>${para.map(inline).join("<br>")}</p>`);
    para = [];
  };
  const closeList = () => {
    if (list) out.push(`</${list}>`);
    list = null;
  };
  const closeCallout = () => {
    if (callout.length) out.push(`<div class="callout" role="note">${callout.map(inline).join("<br>")}</div>`);
    callout = [];
  };

  for (const raw of md.replace(/\r\n?/g, "\n").split("\n")) {
    const line = raw.trimEnd();
    const h = /^(#{1,3})\s+(.*)$/.exec(line);
    const ul = /^\s*[-*•]\s+(.*)$/.exec(line);
    const ol = /^\s*\d+[.)]\s+(.*)$/.exec(line);
    const warn = /^!!\s+(.*)$/.exec(line);

    if (warn) {
      closePara();
      closeList();
      callout.push(warn[1]);
      continue;
    }
    closeCallout();
    if (h) {
      closePara();
      closeList();
      const level = h[1].length + 2; // # → h3, ## → h4, ### → h5 (sous le titre de la carte)
      out.push(`<h${level}>${inline(h[2])}</h${level}>`);
    } else if (ul || ol) {
      closePara();
      const kind = ul ? "ul" : "ol";
      if (list !== kind) {
        closeList();
        out.push(`<${kind}>`);
        list = kind;
      }
      out.push(`<li>${inline((ul ?? ol)![1])}</li>`);
    } else if (line.trim() === "") {
      closePara();
      closeList();
    } else {
      closeList();
      para.push(line);
    }
  }
  closePara();
  closeList();
  closeCallout();
  return out.join("");
}
