// Lecteur YouTube intégré (youtube-nocookie.com), piloté par postMessage (API IFrame, enablejsapi=1) :
// pas de script externe à charger. Partagé par le bloc vidéo et la fenêtre compacte.
//
// Une même vidéo peut être ouverte dans les deux fenêtres : quand l'une démarre, l'autre se met en pause
// (événement `video-playing`), pour ne jamais entendre deux fois la même vidéo.
import { on, send } from "../sync";

/** États du lecteur YouTube (API IFrame). */
const PLAYING = 1;
const BUFFERING = 3;

export class YouTubePlayer {
  iframe: HTMLIFrameElement | undefined = $state();
  /** Position de lecture (secondes), remontée par le lecteur. */
  current = $state(0);
  playing = $state(false);
  #blockId: number;

  constructor(blockId: number) {
    this.#blockId = blockId;
  }

  post(func: string, args: unknown[] = []) {
    this.iframe?.contentWindow?.postMessage(JSON.stringify({ event: "command", func, args }), "*");
  }

  play() {
    this.post("playVideo");
  }

  pause() {
    this.post("pauseVideo");
  }

  toggle() {
    if (this.playing) this.pause();
    else this.play();
  }

  seek(t: number) {
    this.post("seekTo", [Math.max(0, t), true]);
    this.play();
  }

  /** Recule de `sec` secondes (sans relancer une vidéo en pause). */
  back(sec: number) {
    this.post("seekTo", [Math.max(0, this.current - sec), true]);
    this.current = Math.max(0, this.current - sec);
  }

  /** À l'événement `load` de l'iframe : demande au lecteur d'envoyer sa position et son état. */
  loaded() {
    this.iframe?.contentWindow?.postMessage(JSON.stringify({ event: "listening", id: this.#blockId, channel: "widget" }), "*");
  }

  /** Écoute le lecteur et l'autre fenêtre. Renvoie la fonction d'arrêt (pour un `$effect`). */
  attach(): () => void {
    const onMessage = (e: MessageEvent) => {
      if (!this.iframe || e.source !== this.iframe.contentWindow) return;
      try {
        const msg = typeof e.data === "string" ? JSON.parse(e.data) : e.data;
        const t = msg?.info?.currentTime;
        if (typeof t === "number") this.current = t;
        const state = msg?.event === "onStateChange" ? msg.info : msg?.info?.playerState;
        if (typeof state === "number") this.#state(state);
      } catch {
        /* message d'un autre format */
      }
    };
    window.addEventListener("message", onMessage);
    const stop = on("video-playing", ({ blockId }) => {
      if (blockId === this.#blockId && this.playing) this.pause();
    });
    return () => {
      window.removeEventListener("message", onMessage);
      stop();
    };
  }

  #state(state: number) {
    const playing = state === PLAYING || (state === BUFFERING && this.playing);
    if (playing && !this.playing) send("video-playing", { blockId: this.#blockId });
    this.playing = playing;
  }
}
