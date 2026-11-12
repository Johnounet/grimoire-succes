// Mini rendu Markdown pour le bloc Texte : titres, gras, italique, listes, liens, paragraphes.
// Le texte est d'abord échappé : aucune balise HTML écrite par l'utilisateur ne passe.
// Volontairement minimal : pas de dépendance, et rien d'exécutable dans le résultat.

const escape = (s: string) =>
  s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#39;");

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
  s = s.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>");
  s = s.replace(/(^|[^*])\*(?!\s)(.+?)\*(?!\*)/g, "$1<em>$2</em>");
  return s;
}

export function renderMarkdown(md: string): string {
  const out: string[] = [];
  let list: "ul" | "ol" | null = null;
  let para: string[] = [];

  const closePara = () => {
    if (para.length) out.push(`<p>${para.map(inline).join("<br>")}</p>`);
    para = [];
  };
  const closeList = () => {
    if (list) out.push(`</${list}>`);
    list = null;
  };

  for (const raw of md.replace(/\r\n?/g, "\n").split("\n")) {
    const line = raw.trimEnd();
    const h = /^(#{1,3})\s+(.*)$/.exec(line);
    const ul = /^\s*[-*•]\s+(.*)$/.exec(line);
    const ol = /^\s*\d+[.)]\s+(.*)$/.exec(line);

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
  return out.join("");
}
