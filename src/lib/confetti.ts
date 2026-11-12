// Pluie de confettis du 100 %, aux couleurs de la jaquette du jeu. Canvas simple, sans dépendance.

/** Couleurs de repli (et touche dorée ajoutée à chaque palette). */
const GOLD = "#f2c14e";
const FALLBACK = [GOLD, "#ffe08a", "#4f7cff", "#b07cff", "#4cc38a", "#ffffff"];

function hsl(r: number, g: number, b: number): [number, number, number] {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  const d = max - min;
  const s = d === 0 ? 0 : d / (1 - Math.abs(2 * l - 1));
  let h = 0;
  if (d) {
    if (max === r) h = ((g - b) / d) % 6;
    else if (max === g) h = (b - r) / d + 2;
    else h = (r - g) / d + 4;
  }
  return [(h * 60 + 360) % 360, s, l];
}

/** Couleurs dominantes et vives d'une image (jaquette). Repli doré si l'image est illisible. */
export async function coverPalette(src: string | null): Promise<string[]> {
  if (!src) return FALLBACK;
  try {
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.src = src;
    await img.decode();
    const w = 24;
    const h = 36;
    const canvas = document.createElement("canvas");
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext("2d", { willReadFrequently: true })!;
    ctx.drawImage(img, 0, 0, w, h);
    const data = ctx.getImageData(0, 0, w, h).data; // lève une erreur si l'image n'est pas lisible (CORS)

    // Teintes regroupées par tranches de 30° ; on garde les pixels assez colorés et lumineux
    const buckets = new Map<number, { n: number; r: number; g: number; b: number }>();
    for (let i = 0; i < data.length; i += 4) {
      const [r, g, b] = [data[i], data[i + 1], data[i + 2]];
      const [hue, s, l] = hsl(r, g, b);
      if (s < 0.25 || l < 0.2 || l > 0.9) continue;
      const k = Math.floor(hue / 30);
      const e = buckets.get(k) ?? { n: 0, r: 0, g: 0, b: 0 };
      e.n++;
      e.r += r;
      e.g += g;
      e.b += b;
      buckets.set(k, e);
    }
    const colors = [...buckets.values()]
      .filter((e) => e.n >= 8)
      .sort((a, b) => b.n - a.n)
      .slice(0, 5)
      .map((e) => {
        // Moyenne de la tranche, éclaircie pour bien ressortir sur le fond sombre
        const lift = (v: number) => Math.round(Math.min(255, (v / e.n) * 1.15 + 20));
        return `rgb(${lift(e.r)}, ${lift(e.g)}, ${lift(e.b)})`;
      });
    return colors.length >= 2 ? [...colors, GOLD, "#ffffff"] : FALLBACK;
  } catch {
    return FALLBACK;
  }
}

interface Piece {
  x: number;
  y: number;
  vx: number;
  vy: number;
  rot: number;
  vrot: number;
  wobble: number;
  size: number;
  color: string;
  round: boolean;
}

/** Lance la pluie dans `canvas` (plein écran). Renvoie la fonction d'arrêt. */
export function rain(canvas: HTMLCanvasElement, colors: string[], durationMs = 3500): () => void {
  const ctx = canvas.getContext("2d")!;
  const dpr = window.devicePixelRatio || 1;
  let W = 0;
  let H = 0;
  const resize = () => {
    W = canvas.clientWidth;
    H = canvas.clientHeight;
    canvas.width = W * dpr;
    canvas.height = H * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  };
  resize();
  window.addEventListener("resize", resize);

  const pieces: Piece[] = [];
  const spawn = (n: number, burst = false) => {
    for (let i = 0; i < n; i++) {
      const fromLeft = Math.random() < 0.5;
      pieces.push({
        // Rafale de départ : deux canons en bas des côtés ; ensuite : pluie depuis le haut
        x: burst ? (fromLeft ? -10 : W + 10) : Math.random() * W,
        y: burst ? H * 0.75 : -20 - Math.random() * 40,
        vx: burst ? (fromLeft ? 1 : -1) * (6 + Math.random() * 9) : (Math.random() - 0.5) * 2,
        vy: burst ? -(9 + Math.random() * 9) : 2 + Math.random() * 3,
        rot: Math.random() * Math.PI,
        vrot: (Math.random() - 0.5) * 0.3,
        wobble: Math.random() * Math.PI * 2,
        size: 6 + Math.random() * 7,
        color: colors[Math.floor(Math.random() * colors.length)],
        round: Math.random() < 0.25,
      });
    }
  };
  spawn(140, true);

  const start = performance.now();
  let last = start;
  let frame = 0;
  const step = (t: number) => {
    const dt = Math.min(2.5, (t - last) / 16.67);
    last = t;
    if (t - start < durationMs) spawn(Math.random() < 0.5 ? 2 : 1);

    ctx.clearRect(0, 0, W, H);
    for (let i = pieces.length - 1; i >= 0; i--) {
      const p = pieces[i];
      p.vy = Math.min(p.vy + 0.18 * dt, 4.5); // gravité, vitesse limite (les confettis flottent)
      p.vx *= Math.pow(0.97, dt);
      p.wobble += 0.08 * dt;
      p.x += (p.vx + Math.sin(p.wobble) * 0.8) * dt;
      p.y += p.vy * dt;
      p.rot += p.vrot * dt;
      if (p.y > H + 30) {
        pieces.splice(i, 1);
        continue;
      }
      ctx.save();
      ctx.translate(p.x, p.y);
      ctx.rotate(p.rot);
      ctx.fillStyle = p.color;
      if (p.round) {
        ctx.beginPath();
        ctx.arc(0, 0, p.size / 2.5, 0, Math.PI * 2);
        ctx.fill();
      } else {
        // Le cosinus fait « tourner » le papier sur lui-même
        ctx.fillRect(-p.size / 2, (-p.size / 4) * Math.cos(p.wobble), p.size, (p.size / 2) * Math.cos(p.wobble));
      }
      ctx.restore();
    }
    if (pieces.length || t - start < durationMs) frame = requestAnimationFrame(step);
  };
  frame = requestAnimationFrame(step);

  return () => {
    cancelAnimationFrame(frame);
    window.removeEventListener("resize", resize);
    ctx.clearRect(0, 0, W, H);
  };
}
