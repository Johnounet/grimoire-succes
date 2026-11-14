// Lecteur YouTube intégré (youtube-nocookie.com), piloté par postMessage (API IFrame, enablejsapi=1) :
// pas de script externe à charger. Partagé par le bloc vidéo et la fenêtre compacte.
//
// Une même vidéo peut être ouverte dans les deux fenêtres : quand l'une démarre, l'autre se met en pause
// (événement `video-playing`), pour ne jamais entendre deux fois la même vidéo.
//
// Au chargement : volume à 40 % (YouTube démarre à fond) et sous-titres coupés. Un volume changé par
// l'utilisateur est repris par les lecteurs ouverts ensuite dans la même fenêtre.
import { on, send } from "../sync";
import type { YouTubeRef } from "../youtube";
import { saveBlock } from "./store.svelte";
import type { Block } from "./types";

/** États du lecteur YouTube (API IFrame). */
const PLAYING = 1;
const BUFFERING = 3;

/** Dernière position connue de chaque vidéo (secondes), pour la fenêtre en cours : un lecteur refermé
 * puis rouvert reprend là où il en était, même si son composant a été recréé entre-temps. */
const positions = new Map<number, number>();

/** Volume (0 à 100) donné à chaque lecteur chargé : 40 %, puis le dernier choisi dans la fenêtre. */
let volume = 40;

export class YouTubePlayer {
  iframe: HTMLIFrameElement | undefined = $state();
  /** Position de lecture (secondes), remontée par le lecteur. */
  current = $state(0);
  playing = $state(false);
  /** Titre et identifiant de la vidéo chargée, donnés par le lecteur. */
  title = $state("");
  videoId = $state("");
  #blockId: number;
  /** Réglages de départ envoyés (volume, sous-titres) : au premier message du lecteur, puis à la lecture. */
  #setupSent = false;
  #setupPlaying = false;
  /** Le lecteur a confirmé notre volume : ses changements suivants viennent de l'utilisateur. */
  #volumeApplied = false;

  /** `remember` : reprend à la dernière position connue de cette vidéo dans la fenêtre (mode compact). */
  constructor(blockId: number, remember = false) {
    this.#blockId = blockId;
    if (remember) this.current = positions.get(blockId) ?? 0;
  }

  #setCurrent(t: number) {
    this.current = t;
    positions.set(this.#blockId, t);
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
    this.#setCurrent(Math.max(0, this.current - sec));
  }

  /** À l'événement `load` de l'iframe : demande au lecteur d'envoyer sa position et son état. */
  loaded() {
    this.#setupSent = this.#setupPlaying = this.#volumeApplied = false;
    this.iframe?.contentWindow?.postMessage(JSON.stringify({ event: "listening", id: this.#blockId, channel: "widget" }), "*");
  }

  /** Écoute le lecteur et l'autre fenêtre. Renvoie la fonction d'arrêt (pour un `$effect`). */
  attach(): () => void {
    const onMessage = (e: MessageEvent) => {
      if (!this.iframe || e.source !== this.iframe.contentWindow) return;
      try {
        const msg = typeof e.data === "string" ? JSON.parse(e.data) : e.data;
        if (!this.#setupSent) {
          this.#setupSent = true;
          this.#setup();
        }
        this.#watchVolume(msg?.info?.volume, msg?.info?.muted);
        const t = msg?.info?.currentTime;
        if (typeof t === "number") this.#setCurrent(t);
        const v = msg?.info?.videoData;
        if (typeof v?.title === "string" && v.title) {
          this.title = v.title;
          this.videoId = typeof v.video_id === "string" ? v.video_id : "";
        }
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

  /** Garde dans le bloc le titre donné par le lecteur, s'il est bien celui de la vidéo du bloc. */
  keepTitle(block: Block<"video">, ref: YouTubeRef | null) {
    if (!ref || !this.title || this.videoId !== ref.id || block.data.title === this.title) return;
    block.data.title = this.title;
    saveBlock(block);
  }

  /** Volume et sous-titres de départ. Les sous-titres se chargent au début de la lecture : renvoyé alors. */
  #setup() {
    this.post("setVolume", [volume]);
    this.post("unloadModule", ["captions"]);
    this.post("unloadModule", ["cc"]);
  }

  #watchVolume(v: unknown, muted: unknown) {
    if (typeof v !== "number") return;
    if (!this.#volumeApplied) {
      this.#volumeApplied = Math.round(v) === volume;
      return;
    }
    if (muted !== true) volume = Math.round(v);
  }

  #state(state: number) {
    if (state === PLAYING && !this.#setupPlaying) {
      this.#setupPlaying = true;
      this.#setup();
    }
    const playing = state === PLAYING || (state === BUFFERING && this.playing);
    if (playing && !this.playing) send("video-playing", { blockId: this.#blockId });
    this.playing = playing;
  }
}
