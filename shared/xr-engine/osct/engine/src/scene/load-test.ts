import { getScenes, getTargets } from "../content/game-config";
import { setTargetBudget } from "../utils/prepared-scenes";
import type { LoadTestControl, LoadTestOptions, LoadTestProgress, LoadTestResult } from "../utils/tracker-debug";
import type { ImageTracker } from "./tracker";

/** After a step's window is ready, the engine settles this long before measuring */
const SETTLE_MS = 1000;
/** A page frame longer than this is a visible stutter */
const LONG_FRAME_MS = 50;
/** Covered lens: picture darker than this share of the uncovered brightness; uncovered again above the second */
const COVERED_SHARE = 0.35;
const UNCOVERED_SHARE = 0.65;
/** Waiting for the tester to cover the lens (then the step's remaining covers are skipped) */
const COVER_TIMEOUT_MS = 20000;
/** After uncovering: not found within this time counts as a miss */
const FIND_TIMEOUT_MS = 10000;
/** At the start: waiting for a trigger to be found before measuring anyway */
const FIRST_FIND_TIMEOUT_MS = 30000;

const sleep = (ms: number) => new Promise<void>(resolve => window.setTimeout(resolve, ms));
const nextFrame = () => new Promise<number>(resolve => window.requestAnimationFrame(resolve));

/** Steps of target budgets, capped at what the experience has, without repeats */
export const loadTestSteps = (extras: readonly number[], available: number): number[] =>
  [...new Set(extras.map(extra => Math.min(extra, available)))].sort((a, b) => a - b);

/** Mean brightness (0–255) of the camera picture, from an 8×6 sample */
class Brightness {
  private canvas = document.createElement("canvas");
  private context: CanvasRenderingContext2D | null;

  constructor(private video: HTMLVideoElement) {
    this.canvas.width = 8;
    this.canvas.height = 6;
    this.context = this.canvas.getContext("2d", { willReadFrequently: true });
  }

  read(): number | null {
    if (!this.context || this.video.readyState < 2) return null;
    this.context.drawImage(this.video, 0, 0, 8, 6);
    const { data } = this.context.getImageData(0, 0, 8, 6);
    let sum = 0;
    for (let i = 0; i < data.length; i += 4) sum += 0.299 * data[i] + 0.587 * data[i + 1] + 0.114 * data[i + 2];
    return sum / (data.length / 4);
  }
}

class Stopped extends Error {}

/**
 * Incremental load test of the whole app (debug, `<load-test-panel>`): the window of scenes kept ready next to the
 * current one gets a growing **target budget** – 0, 5, 10, 20, 40, 80, all other targets, nearest scenes first.
 * The app runs as usual meanwhile: the tracker holds the window, the preloader fetches its content, pages switch by
 * themselves or by hand. Per step it measures preparing and extracting the window's new targets, engine and page
 * frame rate, stutter, how steadily the scanned page stays found, and – when the tester covers and uncovers the
 * lens – how fast it is found again. Afterwards the window is the ±n setting again.
 */
export const runLoadTest = (
  tracker: ImageTracker,
  options: LoadTestOptions,
  onProgress: (progress: LoadTestProgress) => void,
): LoadTestControl => {
  let stopped = false;
  const startScene = tracker.currentSceneId;
  const sceneTargets = () => (tracker.currentSceneId ? getTargets(tracker.currentSceneId).length : 0);
  const steps = loadTestSteps(options.extras, Math.max(0, getScenes().reduce((n, s) => n + s.targets.length, 0) - sceneTargets()));
  const results: LoadTestResult[] = [];
  let progress: LoadTestProgress = {
    phase: "idle", message: "", step: 0, steps: steps.length, sceneId: startScene, sceneTargets: sceneTargets(), results,
  };
  const report = (change: Partial<LoadTestProgress>) => {
    progress = { ...progress, ...change, sceneId: tracker.currentSceneId, sceneTargets: sceneTargets(), results: [...results] };
    onProgress(progress);
  };
  const check = () => {
    if (stopped) throw new Stopped();
    if (!tracker.hasCamera) throw new Error("the camera stopped");
  };

  /** Waits for a condition, checked every page frame; false after the timeout */
  const waitFor = async (condition: () => boolean, timeoutMs: number): Promise<boolean> => {
    const until = performance.now() + timeoutMs;
    while (performance.now() < until) {
      check();
      if (condition()) return true;
      await nextFrame();
    }
    return false;
  };

  const measureSteady = async () => {
    const video = tracker.cameraVideo;
    const brightness = video ? new Brightness(video) : null;
    const before = tracker.stats;
    const begun = performance.now();
    let last = begun;
    let pageFrames = 0;
    let longFrames = 0;
    let light = 0;
    let lightSamples = 0;
    while (performance.now() - begun < options.steadyMs) {
      check();
      const now = await nextFrame();
      if (now - last > LONG_FRAME_MS) longFrames++;
      last = now;
      pageFrames++;
      if (pageFrames % 10 === 0) {
        const value = brightness?.read();
        if (value != null) { light += value; lightSamples++; }
      }
    }
    const seconds = (performance.now() - begun) / 1000;
    const after = tracker.stats;
    const frames = after.frames - before.frames;
    return {
      engineFps: frames / seconds,
      pageFps: pageFrames / seconds,
      longFrames,
      foundShare: frames ? (after.framesFound - before.framesFound) / frames : 0,
      lost: after.lost - before.lost,
      brightness: lightSamples ? light / lightSamples : null,
    };
  };

  /** Cover → uncover → time until a target of the current scene is found again (null: not found in time) */
  const measureFind = async (step: number, cover: number, base: number): Promise<number | null | "skipped"> => {
    const video = tracker.cameraVideo;
    if (!video) return "skipped";
    const brightness = new Brightness(video);
    const light = () => brightness.read() ?? base;
    report({ phase: "cover", step, message: `Cover the lens with your hand (${cover}/${options.covers})` });
    const covered = await waitFor(() => light() < base * COVERED_SHARE && tracker.foundCount === 0, COVER_TIMEOUT_MS);
    if (!covered) return "skipped";
    report({ phase: "uncover", step, message: `Now uncover it – keep the phone on the page (${cover}/${options.covers})` });
    if (!await waitFor(() => light() > base * UNCOVERED_SHARE, COVER_TIMEOUT_MS)) return "skipped";
    const uncovered = performance.now();
    return await waitFor(() => tracker.foundCount > 0, FIND_TIMEOUT_MS) ? performance.now() - uncovered : null;
  };

  const run = async () => {
    if (!tracker.currentSceneId) throw new Error("no scene is tracked – start scanning first");
    report({ phase: "settle", message: "Hold the phone on a trigger of this page until it is found" });
    await waitFor(() => tracker.foundCount > 0, FIRST_FIND_TIMEOUT_MS);
    for (let i = 0; i < steps.length; i++) {
      const extra = steps[i];
      const step = i + 1;
      report({ phase: "loading", step, message: `Window: ${extra} more targets – loading (pages switch as usual)` });
      setTargetBudget(extra);
      const { prepareMs, extractMs } = await tracker.whenSettled();
      check();
      report({ phase: "settle", step, message: "Keep the phone still on a page with a trigger" });
      await sleep(SETTLE_MS);
      report({ phase: "steady", step, message: `Measuring ${Math.round(options.steadyMs / 1000)} s – keep the phone still on the page` });
      const steady = await measureSteady();
      const findMs: number[] = [];
      let misses = 0;
      for (let cover = 1; cover <= options.covers && steady.brightness; cover++) {
        const found = await measureFind(step, cover, steady.brightness);
        if (found === "skipped") break;
        if (found === null) misses++;
        else findMs.push(found);
        await sleep(500);
      }
      const memory = (performance as Performance & { memory?: { usedJSHeapSize: number } }).memory;
      results.push({
        extra,
        sceneId: tracker.currentSceneId ?? "–",
        loaded: tracker.heldTargets,
        scenes: tracker.heldScenes,
        prepareMs,
        extractMs,
        engineFps: steady.engineFps,
        pageFps: steady.pageFps,
        longFrames: steady.longFrames,
        foundShare: steady.foundShare,
        lost: steady.lost,
        findMs,
        misses,
        heapMb: memory ? memory.usedJSHeapSize / 1048576 : null,
      });
      report({ phase: "settle", step, message: `Step ${step} done` });
    }
  };

  void run().then(
    () => report({ phase: "done", message: "Done – the window is the ±n setting again" }),
    error => report(error instanceof Stopped
      ? { phase: "stopped", message: "Stopped – the window is the ±n setting again" }
      : { phase: "error", message: `Stopped: ${(error as Error)?.message ?? error}` }),
  ).finally(() => setTargetBudget(null));

  return { stop: () => { stopped = true; } };
};
