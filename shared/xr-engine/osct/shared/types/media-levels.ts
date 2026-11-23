/**
 * A picture, a video or a sound in several quality levels (content schema 4.3, #331 step 3 – the media part of #13),
 * the same shape as a model's map sizes (`./textures.ts`): the reader's media quality chooses the level it loads.
 *
 * - An entity's image / video / audio file (`AssetData.levels`) and an entry's colour picture (`EntryData.imageLevels`).
 * - The level is a number whose meaning goes by the kind: an image's **longest side** in px, a video's **height** in px,
 *   a sound's **bitrate** in kbps (`MEDIA_LEVELS`). Each level once; at least one.
 * - The medium's own `src` stays: it is what an older app loads (4.2 and before), and a medium without levels is one
 *   level. Put the level that suits every reader there (the middle one).
 *
 * Levels are made beforehand – pictures in the editor's browser, videos before the upload (never encoded in the browser),
 * sounds uploaded per level; the release step (#304) later turns them into the delivery formats.
 */

import { isObject } from "../guards/primitives";

/** The levels each kind may hold: image px (longest side), video px (height), audio kbps */
export const MEDIA_LEVELS = {
  image: [512, 1024, 2048],
  video: [480, 720, 1080],
  audio: [64, 128, 192],
} as const;

/** The asset types that can have levels */
export type LevelledKind = keyof typeof MEDIA_LEVELS;
export const LEVELLED_KINDS = Object.keys(MEDIA_LEVELS) as LevelledKind[];
export const isLevelledKind = (value: unknown): value is LevelledKind => typeof value === "string" && value in MEDIA_LEVELS;

/** What a kind's level number measures, for messages and the editor */
export const MEDIA_LEVEL_UNIT: Record<LevelledKind, string> = { image: "px", video: "p", audio: "kbps" };

/** One level of a medium: the file at that quality */
export interface MediaLevelData {
  level: number;
  src: string;
}

/**
 * The level of a medium to load at a wanted level: the largest one that is not larger, else the smallest there is (the
 * nearest one when every level is above). `undefined` for no levels.
 */
export function mediaLevelFor(levels: readonly MediaLevelData[] | undefined, wanted: number): MediaLevelData | undefined {
  const sorted = [...(levels ?? [])].sort((a, b) => a.level - b.level);
  return sorted.filter(entry => entry.level <= wanted).pop() ?? sorted[0];
}

/** The file to load at a wanted level: the chosen level's, or the medium's own `src` when it has no levels */
export const mediaSrcFor = (src: string, levels: readonly MediaLevelData[] | undefined, wanted: number): string =>
  mediaLevelFor(levels, wanted)?.src ?? src;

/** What is wrong with a medium's levels (config guard, content build, the editor's save): one line per problem */
export function mediaLevelProblems(raw: unknown, kind: unknown): string[] {
  if (!isLevelledKind(kind)) return [`only an image, a video or a sound has levels (not ${JSON.stringify(kind)})`];
  if (!Array.isArray(raw) || !raw.length) return ["expected a list of at least one level"];
  const allowed: readonly number[] = MEDIA_LEVELS[kind];
  const problems: string[] = [];
  const seen = new Set<unknown>();
  raw.forEach((entry, i) => {
    if (!isObject(entry)) return void problems.push(`[${i}]: expected an object { level, src }`);
    if (typeof entry.level !== "number" || !allowed.includes(entry.level)) {
      problems.push(`[${i}].level: ${JSON.stringify(entry.level)} is not one of ${allowed.join(", ")} (${kind}, ${MEDIA_LEVEL_UNIT[kind]})`);
    } else if (seen.has(entry.level)) problems.push(`[${i}].level: ${entry.level} is there twice`);
    seen.add(entry.level);
    if (typeof entry.src !== "string" || !entry.src) problems.push(`[${i}].src: expected a file`);
  });
  return problems;
}
