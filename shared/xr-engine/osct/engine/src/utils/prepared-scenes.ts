import type { Scene } from "../types/index";
import { getScenes, getTargets } from "../content/game-config";
import { readSwitch, writeSwitch } from "./device-storage";

/**
 * The window of scenes kept ready next to the current one: their targets stay loaded in the engine
 * (their pages are recognised at once and switch the scene by themselves) and their content is preloaded
 * (`PreloaderService.preloadScenes()`). Nearest scenes first (the next before the previous).
 *
 * Setting (debug overlay, this device only – in the app's storage, `device-storage.ts`):
 * - ±n scenes (0, 1, 2) or all: a fixed window that slides with the current scene (`prepareTargets()`).
 *   **±2 is the default**: the next two scenes on either side switch by themselves. Only scenes with targets
 *   count – one without (all its targets switched off, or none yet) is skipped, it has nothing to recognise, so a
 *   page beyond it is still in reach.
 * - **grow**: the current scene first, then the nearest scenes are added in idle time, a few targets at a time,
 *   until the whole experience is held (`IImageTracker.growTargets()`). Nothing is dropped on a switch. Content is
 *   preloaded for ±1. Not the default: with the whole experience held, pages of a printout were no longer found.
 * The engine load test sets a **target budget** instead (the nearest scenes until their targets reach it) while
 * it runs; the window follows every change at once.
 * The trigger trial overrides the setting with ±1 while its panel is open (`setWindowOverride()`, not stored), so the
 * next page is found by itself while the sheet is scanned in order, and nothing beyond the neighbours is prepared or
 * preloaded: a target that is not found is down to the engine and the printout, not to work in the background.
 */
export type PreparedWindowSetting = "grow" | number;
export const PREPARED_WINDOW_SETTINGS: readonly PreparedWindowSetting[] = ["grow", 0, 1, 2, Infinity];
const RANGE_KEY = "ar-prepared-scenes";
const DEFAULT_WINDOW: PreparedWindowSetting = 2;
/** Content preload in grow mode: the neighbours (their media add up – the whole experience stays a separate step) */
const GROW_CONTENT_RANGE = 1;

let windowOverride: PreparedWindowSetting | null = null;

/** The setting in force: the trial's override while it is open, else the stored one */
export const getPreparedWindowSetting = (): PreparedWindowSetting => windowOverride ?? getStoredWindowSetting();

/** The setting chosen in the debug overlay (stored on this device), regardless of an override */
export const getStoredWindowSetting = (): PreparedWindowSetting => {
  const stored = readSwitch(RANGE_KEY);
  if (stored === "grow") return "grow";
  if (stored === null) return DEFAULT_WINDOW;
  const range = stored === "all" ? Infinity : Number(stored);
  return PREPARED_WINDOW_SETTINGS.includes(range) ? range : DEFAULT_WINDOW;
};

/** The window as ±n scenes: the setting's range, or the neighbours in grow mode (content preload) */
export const getPreparedSceneRange = (): number => {
  const setting = getPreparedWindowSetting();
  return setting === "grow" ? GROW_CONTENT_RANGE : setting;
};

/** The next setting in `PREPARED_WINDOW_SETTINGS`, stored – applies at once (the scene re-prepares its window) */
export const cyclePreparedSceneRange = (): PreparedWindowSetting => {
  const next = PREPARED_WINDOW_SETTINGS[(PREPARED_WINDOW_SETTINGS.indexOf(getStoredWindowSetting()) + 1) % PREPARED_WINDOW_SETTINGS.length];
  writeSwitch(RANGE_KEY, next === Infinity ? "all" : String(next));
  windowListeners.forEach(listener => listener());
  return next;
};

let targetBudget: number | null = null;
const windowListeners = new Set<() => void>();

/** Overrides the stored setting until it is set back to null (the trial panel: ±1) – applies at once, not stored */
export const setWindowOverride = (setting: PreparedWindowSetting | null): void => {
  if (setting === windowOverride) return;
  windowOverride = setting;
  windowListeners.forEach(listener => listener());
};

export const getWindowOverride = (): PreparedWindowSetting | null => windowOverride;

/** Target budget of the load test (other scenes' targets held next to the current scene), null = ±n range */
export const getTargetBudget = (): number | null => targetBudget;

export const setTargetBudget = (budget: number | null): void => {
  if (budget === targetBudget) return;
  targetBudget = budget;
  windowListeners.forEach(listener => listener());
};

/** The tracker grows its window towards the whole experience (grow setting, no load test budget) */
export const isGrowingWindow = (): boolean => targetBudget === null && getPreparedWindowSetting() === "grow";

/** Called when the window changes outside a scene switch (setting changed, the load test's budget) */
export const subscribePreparedWindow = (listener: () => void): (() => void) => {
  windowListeners.add(listener);
  return () => windowListeners.delete(listener);
};

/** The other scenes, nearest first – at the same distance the next before the previous */
export const scenesByDistance = (sceneId: string): Scene[] => {
  const scenes = getScenes();
  const index = scenes.findIndex(scene => scene.id === sceneId);
  if (index === -1) return [];
  const ordered: Scene[] = [];
  for (let distance = 1; distance < scenes.length; distance++) {
    if (index + distance < scenes.length) ordered.push(scenes[index + distance]);
    if (index - distance >= 0) ordered.push(scenes[index - distance]);
  }
  return ordered;
};

/**
 * The scenes kept ready next to `sceneId` (target budget while the load test runs, else ±range), nearest first.
 * Only scenes with targets count: one without any is left out and neither uses up the range nor the distance, so
 * the next scene with targets comes first even when an empty one lies between.
 */
export const getPreparedWindow = (sceneId: string, range = getPreparedSceneRange(), budget = targetBudget): Scene[] => {
  const scenes = getScenes();
  const index = scenes.findIndex(scene => scene.id === sceneId);
  if (index === -1) return [];
  const withTargets = (scene: Scene) => getTargets(scene.id).length > 0;
  const ahead = scenes.slice(index + 1).filter(withTargets);
  const behind = scenes.slice(0, index).reverse().filter(withTargets);
  const ordered: Scene[] = [];
  for (let step = 0; step < Math.max(ahead.length, behind.length); step++) {
    if (budget === null && step >= range) break;
    if (step < ahead.length) ordered.push(ahead[step]);
    if (step < behind.length) ordered.push(behind[step]);
  }
  if (budget === null) return ordered;
  const window: Scene[] = [];
  let targets = 0;
  for (const scene of ordered) {
    if (targets >= budget) break;
    window.push(scene);
    targets += getTargets(scene.id).length;
  }
  return window;
};
