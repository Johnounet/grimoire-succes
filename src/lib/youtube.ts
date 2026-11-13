// Liens YouTube : identifiant de la vidéo et temps de départ éventuel.

export interface YouTubeRef {
  id: string;
  start: number;
}

const ID = /^[\w-]{11}$/;

/** Accepte youtube.com/watch?v=…, youtu.be/…, /shorts/…, /embed/…, /live/… (avec ou sans https). */
export function parseYouTube(input: string): YouTubeRef | null {
  let url: URL;
  try {
    const s = input.trim();
    url = new URL(/^https?:\/\//i.test(s) ? s : `https://${s}`);
  } catch {
    return null;
  }
  const host = url.hostname.replace(/^(www\.|m\.|music\.)/, "");
  let id: string | null = null;
  if (host === "youtu.be") id = url.pathname.slice(1).split("/")[0];
  else if (host === "youtube.com" || host === "youtube-nocookie.com") {
    id = url.searchParams.get("v");
    const m = /^\/(shorts|embed|live|v)\/([\w-]{11})/.exec(url.pathname);
    if (!id && m) id = m[2];
  }
  if (!id || !ID.test(id)) return null;
  return { id, start: parseStart(url.searchParams.get("t") ?? url.searchParams.get("start")) };
}

/** « 90 », « 90s », « 1m30s », « 1h2m3s » → secondes. */
function parseStart(t: string | null): number {
  if (!t) return 0;
  if (/^\d+$/.test(t)) return Number(t);
  const m = /^(?:(\d+)h)?(?:(\d+)m)?(?:(\d+)s)?$/.exec(t);
  return m ? Number(m[1] ?? 0) * 3600 + Number(m[2] ?? 0) * 60 + Number(m[3] ?? 0) : 0;
}

/** `autoplay` : lecture dès le chargement (vidéo lancée depuis le mode compact). */
export function embedUrl(ref: YouTubeRef, autoplay = false): string {
  const params = new URLSearchParams({
    enablejsapi: "1",
    rel: "0",
    origin: window.location.origin,
  });
  if (ref.start) params.set("start", String(ref.start));
  if (autoplay) params.set("autoplay", "1");
  return `https://www.youtube-nocookie.com/embed/${ref.id}?${params}`;
}

export const watchUrl = (ref: YouTubeRef, at = 0) =>
  `https://www.youtube.com/watch?v=${ref.id}${at > 0 ? `&t=${Math.floor(at)}s` : ""}`;
