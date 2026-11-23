import { cueAt, parseWebVtt, type SubtitleCue } from "@shared/text/webvtt";
import { getSubtitleTrack } from "../content/game-config";

/**
 * The subtitle line of the scene: while a video of the scene plays, the cue of its subtitle file at the video's own
 * time – the video is the clock, so the line can not drift from the sound. The scene asks every frame (`update`) and
 * listens to the line only when it changes (`report`: the text, or null for none). Which file a video has is the content's
 * (`getSubtitleTrack`: the track of the app's language, with its fallbacks); a file is fetched once, when its video
 * first plays. The scene shows nothing itself: the app draws the line.
 */

/** A video of the scene as the subtitles see it: its file and the element that plays it */
export interface SubtitledVideo {
  src: string;
  element: Pick<HTMLVideoElement, "paused" | "ended" | "currentTime">;
}

type Fetcher = (url: string) => Promise<string>;

const fetchText: Fetcher = async url => {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return response.text();
};

export class Subtitles {
  /** The cues of every file asked for, by URL – empty while it loads and when it failed */
  private files = new Map<string, SubtitleCue[]>();
  private line: string | null = null;

  constructor(
    private language: () => string,
    private report: (text: string | null) => void,
    private fetcher: Fetcher = fetchText,
  ) {}

  private cues(url: string): SubtitleCue[] {
    const known = this.files.get(url);
    if (known) return known;
    this.files.set(url, []);
    void this.fetcher(url)
      .then(text => this.files.set(url, parseWebVtt(text)))
      .catch(error => console.warn(`[subtitles] Could not load ${url}:`, error));
    return [];
  }

  /** Every frame: the line of the playing videos – of several, the cue that started last */
  update(videos: Iterable<SubtitledVideo>): void {
    let current: { text: string; since: number } | null = null;
    const language = this.language();
    for (const { src, element } of videos) {
      if (element.paused || element.ended) continue;
      const track = getSubtitleTrack(src, language);
      if (!track) continue;
      const cue = cueAt(this.cues(track), element.currentTime);
      if (!cue) continue;
      const since = element.currentTime - cue.start;
      if (!current || since < current.since) current = { text: cue.text, since };
    }
    this.show(current?.text ?? null);
  }

  /** No video plays any more (the scene paused, the scene changed): the line goes */
  clear(): void {
    this.show(null);
  }

  private show(text: string | null): void {
    if (text === this.line) return;
    this.line = text;
    this.report(text);
  }
}
