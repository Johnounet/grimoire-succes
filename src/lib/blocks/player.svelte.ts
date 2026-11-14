// Lecteur YouTube intégré (youtube-nocookie.com), piloté par postMessage (API IFrame, enablejsapi=1) :
// pas de script externe à charger. Partagé par le bloc vidéo et la fenêtre compacte.
//
// Une même vidéo peut être ouverte dans les deux fenêtres : quand l'une démarre, l'autre se met en pause
// (événement `video-playing`), pour ne jamais entendre deux fois la même vidéo. Les deux copies partagent
// leur position : celle en pause suit celle qui joue (`video-position`, chaque seconde pendant la lecture,
// et à l'arrêt), pour reprendre là où l'autre en est.
//
// Au chargement : volume à 40 % (YouTube démarre à fond) et sous-titres coupés. Un volume changé par
// l'utilisateur est repris par les lecteurs ouverts ensuite dans la même fenêtre.
import { on, send } from "../sync";
import type { YouTubeRef } from "../youtube";
import { saveBlock } from "./store.svelte";
import type { Block } from "./types";

/** États du lecteur YouTube (API IFrame). */
const PLAYING = 1;
const PAUSED = 2;
const BUFFERING = 3;
/** Intervalle d'envoi de la position pendant la lecture (ms). */
const SHARE_MS = 1000;

/** Dernière position connue de chaque bloc vidéo (secondes), pour la fenêtre en cours : un lecteur refermé
 * puis rouvert reprend là où il en était, même si son composant a été recréé entre-temps. Gardée avec
 * l'identifiant de la vidéo : un bloc dont le lien a changé repart du début. */
const positions = new Map<number, { video: string; t: number }>();

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
  /** Vidéo suivie pour la reprise (mode compact) ; vide : position non gardée. */
  #video = "";
  /** Réglages de départ envoyés (volume, sous-titres) : au premier message du lecteur, puis à la lecture. */
  #setupSent = false;
  #setupPlaying = false;
  /** Le lecteur a confirmé notre volume : ses changements suivants viennent de l'utilisateur. */
  #volumeApplied = false;
  /** Le lecteur est à l'état « pause » : seul cas où un `seekTo` ne le fait pas jouer (pas avant la
   * première lecture, ni à la fin de la vidéo). */
  #paused = false;
  /** Position reçue de l'autre fenêtre, à appliquer au lecteur (avant cela, ses positions sont ignorées). */
  #pending: number | null = null;
  #lastShared = 0;
  /** Mise en pause parce que l'autre fenêtre démarre : sa position fait foi, ne pas envoyer la nôtre. */
  #yielding = false;

  constructor(blockId: number) {
    this.#blockId = blockId;
  }

  /** Mode compact : garde la position de la vidéo `video` (identifiant YouTube) et reprend la dernière
   * connue dans la fenêtre. À rappeler quand le lien du bloc change (repart alors du début). */
  follow(video: string) {
    if (video === this.#video) return;
    this.#video = video;
    const saved = positions.get(this.#blockId);
    this.current = video && saved?.video === video ? saved.t : 0;
  }

  #setCurrent(t: number) {
    this.current = t;
    if (this.#video) positions.set(this.#blockId, { video: this.#video, t });
  }

  post(func: string, args: unknown[] = []) {
    this.iframe?.contentWindow?.postMessage(JSON.stringify({ event: "command", func, args }), "*");
  }

  play() {
    this.#applyPending();
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
    this.#pending = null;
    this.post("seekTo", [Math.max(0, t), true]);
    this.#setCurrent(Math.max(0, t));
    this.play();
  }

  /** Recule de `sec` secondes (sans relancer une vidéo en pause). */
  back(sec: number) {
    const t = Math.max(0, this.current - sec);
    this.#setCurrent(t);
    if (this.playing || this.#paused) {
      this.#pending = null;
      this.post("seekTo", [t, true]);
    } else {
      // Vidéo terminée ou jamais lancée : un `seekTo` la relancerait, la position attend la lecture
      this.#pending = t;
    }
    this.#share(!this.playing);
  }

  /** À l'événement `load` de l'iframe : demande au lecteur d'envoyer sa position et son état. */
  loaded() {
    this.#setupSent = this.#setupPlaying = this.#volumeApplied = this.#paused = false;
    // Nouveau lecteur, ouvert à la position voulue (`start`) : une position reçue avant ne vaut plus
    this.#pending = null;
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
        if (typeof t === "number" && this.#pending === null) {
          this.#setCurrent(t);
          if (this.playing && Date.now() - this.#lastShared >= SHARE_MS) this.#share(false);
        }
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
    const stopPlaying = on("video-playing", ({ blockId, t }) => {
      if (blockId !== this.#blockId) return;
      this.#receive(t, true);
      if (this.playing) {
        this.#yielding = true;
        this.pause();
      }
    });
    const stopPosition = on("video-position", ({ blockId, t, settle }) => {
      if (blockId === this.#blockId && !this.playing) this.#receive(t, settle);
    });
    return () => {
      window.removeEventListener("message", onMessage);
      stopPlaying();
      stopPosition();
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

  /** Envoie notre position à l'autre fenêtre. */
  #share(settle: boolean) {
    this.#lastShared = Date.now();
    send("video-position", { blockId: this.#blockId, t: this.current, settle });
  }

  /** Position de l'autre fenêtre : affichée tout de suite, appliquée au lecteur dès que possible
   * (tout de suite s'il est en pause ; sinon au prochain démarrage, car un `seekTo` sur un lecteur
   * jamais lancé ou arrivé à la fin le ferait jouer). Sans lecteur ouvert, la position sert à la réouverture. */
  #receive(t: number, settle: boolean) {
    this.#setCurrent(t);
    if (!this.iframe) return;
    this.#pending = t;
    if (settle && this.#paused) this.#applyPending();
  }

  #applyPending() {
    if (this.#pending === null) return;
    this.post("seekTo", [this.#pending, true]);
    this.#pending = null;
  }

  #state(state: number) {
    if (state === PLAYING && !this.#setupPlaying) {
      this.#setupPlaying = true;
      this.#setup();
    }
    if (state === PLAYING) this.#applyPending();
    const wasPaused = this.#paused;
    this.#paused = state === PAUSED;
    const playing = state === PLAYING || (state === BUFFERING && this.playing);
    if (playing && !this.playing) {
      this.#yielding = false;
      send("video-playing", { blockId: this.#blockId, t: this.current });
    } else if (!playing && this.playing) {
      // Arrêt (pause, fin) décidé ici : l'autre copie se place à notre position
      if (!this.#yielding) this.#share(true);
      this.#yielding = false;
    }
    // Arrivée en pause (dont la pause demandée par l'autre fenêtre) : une position reçue entre-temps
    // s'applique maintenant. Pas à la fin de la vidéo : le `seekTo` la relancerait.
    if (this.#paused && !wasPaused) this.#applyPending();
    this.playing = playing;
  }
}
