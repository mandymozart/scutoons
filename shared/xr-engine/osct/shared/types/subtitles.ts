/**
 * Subtitles of a video (content schema 2.31, #257): the spoken line of a video that carries its own sound, as WebVTT
 * files – one per language. A subtitle file belongs to a video **by its name**: `<video name>.<language>.vtt` next to
 * the video (`speak.mp4` → `speak.en.vtt`, `speak.fr.vtt`). Nobody fills the pairing in: the content build and the
 * editor's API read it from the names and write it into the configuration's `subtitles` list, so the engine never
 * guesses a URL. The list is keyed by the video's file, not by a node – every use of the video carries its subtitles.
 * The cues' grammar: `../text/webvtt.ts`.
 */

import { isObject } from "../guards/primitives";

/** The subtitle files of one video */
export interface SubtitleData {
  /** The video as the nodes name it (their asset's `src`) */
  media: string;
  /** Language (`en`, `fr`, `pt-BR`) → the WebVTT file */
  tracks: Record<string, string>;
}

/** The extension of a subtitle file */
export const SUBTITLE_EXTENSION = ".vtt";

/** A language as a subtitle file names it: a two- or three-letter code with optional subtags (`en`, `nl`, `pt-BR`) */
export const SUBTITLE_LANGUAGE_PATTERN = /^[a-z]{2,3}(-[A-Za-z0-9]{2,8})*$/;

/** The video extensions a subtitle file can belong to (the contract's video files) */
const VIDEO_EXTENSION = /\.(mp4|webm|mov|m4v)$/i;

/** A path's or URL's file name: after the last slash, without a query or fragment */
const fileName = (path: string): string => path.replace(/[?#].*$/, "").replace(/^.*[\\/]/, "");

/**
 * What a subtitle file's name says: the name of its video without the extension, and the language –
 * `speak.en.vtt` → { video: "speak", language: "en" }. Null for any other name.
 */
export const subtitleFileOf = (path: string): { video: string; language: string } | null => {
  const match = /^(.+)\.([^.]+)\.vtt$/i.exec(fileName(path));
  if (!match || !SUBTITLE_LANGUAGE_PATTERN.test(match[2])) return null;
  return { video: match[1], language: match[2] };
};

/** The name a video's subtitle files start with: its file name without the extension; null when it is no video */
export const subtitleNameOf = (path: string): string | null => {
  const name = fileName(path);
  return VIDEO_EXTENSION.test(name) ? name.replace(VIDEO_EXTENSION, "") : null;
};

/**
 * The subtitle list of a set of files: every video that has subtitle files with its name **in its own folder**, its
 * tracks sorted by language. `files` are paths or URLs as the configuration names them.
 */
export const pairSubtitles = (files: readonly string[]): SubtitleData[] => {
  const folder = (path: string) => path.replace(/[?#].*$/, "").replace(/[^\\/]*$/, "");
  const tracks = new Map<string, Record<string, string>>();
  files.forEach(file => {
    const subtitle = subtitleFileOf(file);
    if (!subtitle) return;
    const key = `${folder(file)}${subtitle.video}`;
    tracks.set(key, { ...tracks.get(key), [subtitle.language]: file });
  });
  return files.flatMap(file => {
    const name = subtitleNameOf(file);
    const found = name === null ? undefined : tracks.get(`${folder(file)}${name}`);
    if (!found) return [];
    return [{ media: file, tracks: Object.fromEntries(Object.entries(found).sort(([a], [b]) => a.localeCompare(b))) }];
  });
};

/**
 * The track for a reader's language: that language, else its base (`fr-BE` → `fr`), else a regional variant of it, else
 * English, else the first. Undefined when there is none.
 */
export const pickSubtitleTrack = (tracks: Record<string, string>, language: string): string | undefined => {
  const languages = Object.keys(tracks);
  if (!languages.length) return undefined;
  const wanted = language.toLowerCase();
  const base = wanted.split("-")[0];
  const found =
    languages.find(l => l.toLowerCase() === wanted) ??
    languages.find(l => l.toLowerCase() === base) ??
    languages.find(l => l.toLowerCase().split("-")[0] === base) ??
    languages.find(l => l.toLowerCase() === "en") ??
    languages[0];
  return tracks[found];
};

/** What is wrong with a configuration's `subtitles` (empty = fine) */
export const subtitleProblems = (raw: unknown, path: string): string[] => {
  if (!Array.isArray(raw)) return [`${path}: expected a list`];
  const problems: string[] = [];
  raw.forEach((item, i) => {
    const at = `${path}[${i}]`;
    if (!isObject(item)) {
      problems.push(`${at}: expected an object`);
      return;
    }
    if (typeof item.media !== "string" || !item.media) problems.push(`${at}.media: expected the video's file`);
    if (!isObject(item.tracks)) {
      problems.push(`${at}.tracks: expected { language: file }`);
      return;
    }
    Object.entries(item.tracks).forEach(([language, file]) => {
      if (!SUBTITLE_LANGUAGE_PATTERN.test(language)) problems.push(`${at}.tracks.${language}: expected a language like "en" or "pt-BR"`);
      if (typeof file !== "string" || !file) problems.push(`${at}.tracks.${language}: expected the subtitle file`);
    });
  });
  return problems;
};
