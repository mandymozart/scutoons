/**
 * Debug view of the image tracker (three-free, main chunk): what the engine currently holds, and the load test.
 * The tracker (lazy AR chunk) publishes here; the debug overlay and the load test panel read it. Nothing in the
 * app depends on it.
 */

/** A scene whose targets the engine holds */
export interface EngineScene {
  sceneId: string;
  targets: number;
}

export interface EngineState {
  /** The scene being tracked (its finds go to the app) */
  current: EngineScene | null;
  /** Scenes held next to it (the window, nearest first) */
  held: EngineScene[];
  /** Scenes whose target images are being turned into image targets (not in the engine yet) */
  preparing: string[];
  /** Targets in the engine, and how many of them are extracted (can be found) */
  targets: number;
  ready: number;
  /** The engine is extracting new targets since (performance.now()), else null */
  extractingSince: number | null;
  /** Target budget of the window while the load test runs, null = the ±n setting */
  budget: number | null;
  /** Grow mode: scenes still to add (0 = done adding, the last batch may still extract), null = not growing */
  growing: number | null;
  /** Target ids the engine reports as found right now – of any loaded scene, not only the current one */
  seen: string[];
  /** The last automatic scene switch: to which scene, the target that caused it, when (Date.now()) */
  lastSwitch: { sceneId: string; targetId: string; at: number } | null;
}

export const EMPTY_ENGINE_STATE: EngineState = {
  current: null, held: [], preparing: [], targets: 0, ready: 0, extractingSince: null, budget: null, growing: null, seen: [], lastSwitch: null,
};

let engineState: EngineState = EMPTY_ENGINE_STATE;
const engineListeners = new Set<(state: EngineState) => void>();

export const getEngineState = (): EngineState => engineState;

export const publishEngineState = (state: EngineState): void => {
  engineState = state;
  engineListeners.forEach(listener => listener(state));
};

export const subscribeEngineState = (listener: (state: EngineState) => void): (() => void) => {
  engineListeners.add(listener);
  return () => engineListeners.delete(listener);
};

// ── Load test ───────────────────────────────────────────────────────────────────────────────────

/** One step of the load test: the window holds the nearest scenes up to `extra` targets next to the current scene */
export interface LoadTestResult {
  extra: number;
  /** The scene being scanned when the step was measured */
  sceneId: string;
  /** Targets in the engine (current scene + extra) and the scenes they belong to */
  loaded: number;
  scenes: number;
  /** Turning the window's new target images into image targets (the app's part, main thread) */
  prepareMs: number;
  /** The engine extracting the new targets' features (one per frame, main thread) */
  extractMs: number;
  /** Frames the engine processed per second, and page frames (requestAnimationFrame) per second */
  engineFps: number;
  pageFps: number;
  /** Page frames that took longer than 50 ms (visible stutter) */
  longFrames: number;
  /** Share of engine frames with a target of the current scene found (0–1) – 1 while held on a page */
  foundShare: number;
  /** Times a found target of the current scene was lost (flicker, or the page left the view) */
  lost: number;
  /** Time from uncovering the lens to the current scene's target found again, per cover */
  findMs: number[];
  /** Covers after which the target was not found within the time limit */
  misses: number;
  /** JS heap in MB (Chrome only), else null */
  heapMb: number | null;
}

export type LoadTestPhase = "idle" | "loading" | "settle" | "steady" | "cover" | "uncover" | "done" | "stopped" | "error";

export interface LoadTestProgress {
  phase: LoadTestPhase;
  /** What the tester should do now (or what went wrong) */
  message: string;
  /** Step running (1-based) and the number of steps */
  step: number;
  steps: number;
  sceneId: string | null;
  /** Targets of the current scene (they are what is measured) */
  sceneTargets: number;
  results: LoadTestResult[];
}

export interface LoadTestOptions {
  /** Target budget of the window per step (nearest scenes first); values above what the experience has are capped */
  extras: number[];
  /** How long each step measures with the phone held on the page */
  steadyMs: number;
  /** Lens covers per step for the re-find time (0 = fully automatic, no tester action) */
  covers: number;
}

export const DEFAULT_LOAD_TEST: LoadTestOptions = {
  extras: [0, 5, 10, 20, 40, 80, Infinity],
  steadyMs: 6000,
  covers: 2,
};

export interface LoadTestControl {
  stop(): void;
}

export type LoadTestRunner = (options: LoadTestOptions, onProgress: (progress: LoadTestProgress) => void) => LoadTestControl;

let loadTestRunner: LoadTestRunner | null = null;
const runnerListeners = new Set<() => void>();

/** Set by the tracker while its camera runs (the test needs the engine), null otherwise */
export const registerLoadTest = (runner: LoadTestRunner | null): void => {
  loadTestRunner = runner;
  runnerListeners.forEach(listener => listener());
};

export const getLoadTest = (): LoadTestRunner | null => loadTestRunner;

export const subscribeLoadTest = (listener: () => void): (() => void) => {
  runnerListeners.add(listener);
  return () => runnerListeners.delete(listener);
};

/** Median of a list of numbers (0 for none) */
export const median = (values: readonly number[]): number => {
  if (!values.length) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
};
