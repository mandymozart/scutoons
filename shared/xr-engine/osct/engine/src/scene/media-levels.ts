import { MEDIA_LEVELS, mediaSrcFor, type LevelledKind, type MediaLevelData } from "@shared/types/media-levels";
import type { MediaQuality } from "../types/graphics";
import { deviceHints, type DeviceHints } from "./model-textures";

/**
 * Which level of a picture, a video or a sound the reader loads (#331 step 3, content schema 4.3): the reader's media
 * quality (Info → Settings) and, for `auto`, the screen, the device and its connection. Pure – the hints come in – so the
 * engine's asset loading, the preloader and the HTML pictures choose the same file.
 *
 * - `low` / `medium` / `high`: the kind's smallest, middle and largest level (`MEDIA_LEVELS`).
 * - `auto` on a data-saving or slow (2g / 3g) connection: the smallest level of every kind.
 * - `auto` otherwise – images: the smallest level that covers the screen's longest side in device pixels; videos: the
 *   smallest height that covers the screen's shorter side in device pixels, at most the middle one on a device with up to
 *   4 GB; sounds: the largest (sound is light).
 * The medium's levels are then searched for the largest not above that, else its smallest (`mediaLevelFor`).
 */

/** The screen in CSS px and its pixel ratio (all optional: tests and the editor's stage) */
export interface MediaHints extends DeviceHints {
  screenWidth?: number;
  screenHeight?: number;
  pixelRatio?: number;
}

export const mediaHints = (): MediaHints => {
  const win = typeof window === "undefined" ? undefined : window;
  return {
    ...deviceHints(),
    screenWidth: win?.screen?.width,
    screenHeight: win?.screen?.height,
    pixelRatio: win?.devicePixelRatio,
  };
};

const isSlow = (hints: MediaHints): boolean => !!hints.saveData || /(^|-)(2g|3g)$/.test(hints.connection ?? "");

/** The level (image px, video height, audio kbps) a quality asks for of one kind on this device */
export function wantedMediaLevel(kind: LevelledKind, quality: MediaQuality | undefined, hints: MediaHints = {}): number {
  const levels: readonly number[] = MEDIA_LEVELS[kind];
  const smallest = levels[0];
  const middle = levels[Math.floor((levels.length - 1) / 2)];
  const largest = levels[levels.length - 1];
  if (quality === "low") return smallest;
  if (quality === "medium") return middle;
  if (quality === "high") return largest;
  if (isSlow(hints)) return smallest;
  const covering = (px: number) => levels.find(level => level >= px) ?? largest;
  const ratio = hints.pixelRatio && hints.pixelRatio > 0 ? hints.pixelRatio : 1;
  const sides = [hints.screenWidth, hints.screenHeight].filter((side): side is number => typeof side === "number" && side > 0);
  switch (kind) {
    case "image":
      return sides.length ? covering(Math.max(...sides) * ratio) : middle;
    case "video": {
      const wanted = sides.length ? covering(Math.min(...sides) * ratio) : middle;
      return hints.memory !== undefined && hints.memory <= 4 ? Math.min(wanted, middle) : wanted;
    }
    case "audio":
      return largest;
  }
}

/** The file of a medium to load: its chosen level's, or its own `src` when it has no levels */
export const chooseMediaSrc = (
  kind: LevelledKind, src: string, levels: readonly MediaLevelData[] | undefined, quality: MediaQuality | undefined, hints: MediaHints = {},
): string => (levels?.length ? mediaSrcFor(src, levels, wantedMediaLevel(kind, quality, hints)) : src);
