/**
 * WebVTT, as far as subtitles need it (`../types/subtitles.ts`): the cues of a file – when each starts and ends, and
 * its text. Read: the `WEBVTT` header, cue identifiers, timestamps with or without hours (a comma for the decimal is
 * taken too), text over several lines. Left out: cue settings (position, alignment), `NOTE`, `STYLE` and `REGION`
 * blocks, and the tags inside a cue's text (`<v Mark>`, `<i>`, `<00:01.000>`) – the text is shown plain.
 */

export interface SubtitleCue {
  /** Seconds from the video's start */
  start: number;
  end: number;
  /** The line shown – several lines keep their line breaks */
  text: string;
}

const TIMESTAMP = /(?:(\d+):)?(\d{1,2}):(\d{2})[.,](\d{1,3})/;
const TIMING = new RegExp(`^\\s*(${TIMESTAMP.source})\\s+-->\\s+(${TIMESTAMP.source})`);

const seconds = (stamp: string): number => {
  const match = TIMESTAMP.exec(stamp);
  if (!match) return NaN;
  const [, hours, minutes, secs, fraction] = match;
  return Number(hours ?? 0) * 3600 + Number(minutes) * 60 + Number(secs) + Number(fraction.padEnd(3, "0")) / 1000;
};

const ENTITIES: Record<string, string> = { "&amp;": "&", "&lt;": "<", "&gt;": ">", "&nbsp;": " ", "&lrm;": "", "&rlm;": "" };

/** A cue's text without its tags, entities resolved */
const plain = (text: string): string =>
  text.replace(/<[^>]*>/g, "").replace(/&(amp|lt|gt|nbsp|lrm|rlm);/g, entity => ENTITIES[entity]).trim();

/** The cues of a WebVTT file, by start time; a block that is no cue is skipped */
export const parseWebVtt = (source: string): SubtitleCue[] => {
  const blocks = source.replace(/^﻿/, "").replace(/\r\n?/g, "\n").split(/\n{2,}/);
  const cues: SubtitleCue[] = [];
  blocks.forEach(block => {
    const lines = block.split("\n").filter(line => line.trim() !== "");
    const at = lines.findIndex(line => TIMING.test(line));
    // The timing is a cue's first line, or its second after an identifier
    if (at < 0 || at > 1) return;
    const [from, to] = lines[at].split("-->");
    const start = seconds(from);
    const end = seconds(to);
    const text = plain(lines.slice(at + 1).join("\n"));
    if (!Number.isFinite(start) || !Number.isFinite(end) || end <= start || !text) return;
    cues.push({ start, end, text });
  });
  return cues.sort((a, b) => a.start - b.start);
};

/** What is wrong with a subtitle file (empty = fine): no header, no cue, a cue that ends before it starts */
export const webVttProblems = (source: string): string[] => {
  const text = source.replace(/^﻿/, "").replace(/\r\n?/g, "\n");
  const problems: string[] = [];
  if (!/^WEBVTT([ \t\n]|$)/.test(text)) problems.push('the first line must be "WEBVTT"');
  text.split(/\n{2,}/).forEach(block => {
    const line = block.split("\n").find(l => TIMING.test(l));
    if (!line) return;
    const [from, to] = line.split("-->");
    if (seconds(to) <= seconds(from)) problems.push(`"${line.trim()}": the cue ends before it starts`);
  });
  if (!parseWebVtt(text).length) problems.push("no cue found");
  return problems;
};

/** The cue shown at a time of the video: the last one that started and has not ended; null between cues */
export const cueAt = (cues: readonly SubtitleCue[], time: number): SubtitleCue | null => {
  let current: SubtitleCue | null = null;
  for (const cue of cues) {
    if (cue.start > time) break;
    if (time < cue.end) current = cue;
  }
  return current;
};
