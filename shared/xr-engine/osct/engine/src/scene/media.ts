/**
 * Playing the scene's media – a video node's video, a video a texture shows – and the editor's hold on them (#275).
 * Every start goes through `playMedia`: with sound where the browser allows it, else muted (no user gesture yet, e.g.
 * iOS), never not at all. While the media are held (the editor's Live switch off – `ArScene.holdClock`) nothing starts:
 * what played stops where it is, what asks to start waits, and when the hold ends exactly those go on. A pause
 * (`pauseMedia`: the page lost, the scene paused) also cancels a start the hold kept back, so a video of a page that
 * went away while the scene stood still stays paused.
 *
 * One hold per page, like the scene: the scene ends it when it is disposed (`releaseMedia`). Media a plugin's code plays
 * itself (`element.play()`) bypass it – the hold pauses them only if they already play when it begins.
 *
 * Sound (#290): the app's sound setting (`setMediaSound` – the reader's Sounds switch) decides whether media play with
 * sound at all; off, they play muted. On, a start the browser refused sound for (no user gesture yet) plays muted and
 * is remembered as *silenced*: the next tap on the scene (`restoreSound`, a user gesture) gives them their sound back.
 */

let held = false;
/** Media that played when the hold began, or asked to start during it: they go on when it ends */
const waiting = new Set<HTMLMediaElement>();
/** Media that play muted only because the browser refused their sound (no user gesture yet) */
const silenced = new Set<HTMLMediaElement>();
/** Media whose sound the current tap gave back (`restoreSound`) – the tap is spent on that, not on pausing them */
let restored = new Set<HTMLMediaElement>();
/** The app's sound setting: may media play with sound (asked at every start) */
let soundOn: () => boolean = () => true;

/** The app's sound setting (the reader's Sounds switch); none = with sound */
export const setMediaSound = (allowed: (() => boolean) | undefined): void => {
  soundOn = allowed ?? (() => true);
};

/**
 * Play – with sound if the app's setting allows it and the browser does, else muted (no user gesture yet, e.g. iOS:
 * silenced until the next tap); never not at all. From where it stopped. While held: started when the hold ends.
 */
export const playMedia = async (element: HTMLMediaElement): Promise<void> => {
  if (held) {
    waiting.add(element);
    return;
  }
  const sound = soundOn();
  element.muted = !sound;
  if (!sound) silenced.delete(element);
  try {
    await element.play();
    if (sound) silenced.delete(element);
  } catch (error) {
    if ((error as DOMException)?.name !== "NotAllowedError" || element.muted) {
      console.warn("[media] Could not play:", error);
      return;
    }
    element.muted = true;
    silenced.add(element);
    await element.play().catch(e => console.warn("[media] Could not play muted:", e));
  }
};

/** Pause – and forget a start the hold kept back */
export const pauseMedia = (element: HTMLMediaElement): void => {
  waiting.delete(element);
  element.pause();
};

/**
 * A tap on the scene (a user gesture – the browser lets media sound now): the silenced media get their sound back,
 * if the app's setting allows it. Those are remembered for this tap (`soundRestored`) until `endTap()`.
 */
export const restoreSound = (): void => {
  restored = new Set();
  if (!silenced.size || !soundOn()) return;
  silenced.forEach(element => {
    element.muted = false;
    restored.add(element);
  });
  silenced.clear();
};

/** The current tap gave this element its sound back (it plays on, with sound) */
export const soundRestored = (element: HTMLMediaElement): boolean => restored.has(element);

/** The tap is handled */
export const endTap = (): void => {
  restored = new Set();
};

/** The element plays muted because the browser refused its sound – a tap gives it back */
export const isSilenced = (element: HTMLMediaElement): boolean => silenced.has(element);

/**
 * Hold the media (`media`: every media element of the scene now) – those that play stop where they are – or end the
 * hold: what played or asked to start goes on, where it stopped. Holding twice or releasing twice changes nothing.
 */
export const holdMedia = (hold: boolean, media: Iterable<HTMLMediaElement> = []): void => {
  if (hold === held) return;
  held = hold;
  if (hold) {
    for (const element of media) {
      if (element.paused) continue;
      waiting.add(element);
      element.pause();
    }
    return;
  }
  const resume = [...waiting];
  waiting.clear();
  // An element disposed meanwhile (its asset released) has left the page
  resume.filter(element => element.isConnected).forEach(element => void playMedia(element));
};

/** The media are held (the editor's Live switch off) */
export const isMediaHeld = (): boolean => held;

/** The scene is gone: no hold, nothing waiting – the next scene starts free */
export const releaseMedia = (): void => {
  held = false;
  waiting.clear();
  silenced.clear();
  restored = new Set();
};
