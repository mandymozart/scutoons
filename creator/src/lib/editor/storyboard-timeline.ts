// The storyboard page's clips lane for the shared Timeline (#126; @scutoons/ui timeline.ts – one linear axis in ms; the
// studio's timeline-lanes.js). Owner 2026-10-19: "every scene is first of all spatial in the webtoon … depending on how
// long a camera is recorded and or trimmed to, thats the length of that clip for that shot", "the other timeline
// underneath the clip only shows those shots that are selected" (DECISIONS "timeline", research/TIMELINE.md §1).
// ONE lane: the PICKED shots back to back in toon order, each ONE clip – the clip of the view mode (like Blender's
// viewport shading: "in our case, it is shot/take/rendered/generated"), falling back generated → rendered → take → still:
//   generated: the shot's generated video – the approved one, else the newest not rejected;
//   rendered:  the newest handheld render of the active take;
//   take:      the active take's kept part (out − in); in Take mode the whole take with its trim (the in / out handles);
//   still:     STILL_MS (owner: "those without a recorded or rendered video will need a default length").
// The clips come from the shot (its takes, inline) and the chapter's generation records; a video's length from the
// record's duration, else from the file's metadata (`known`, read in the browser), else 5 s until it is read (`probe`).
// `clip.from` = where the clip's media is at the item's start (ms): a take's in outside Take mode, else 0.
// Pure – no Svelte, no DOM (test/storyboard-timeline.test.ts).
import type { Shot } from '@scutoons/content/storyboard';
import type { TimelineItem, TimelineLane } from '@scutoons/ui/timeline';
import { shotTake, RENDER_GENERATOR, type TakeIn } from './storyboard-takes.ts';

/** a shot without a recorded / rendered / generated clip (owner: "perhapse 5 seconds or 15 sec" – 5) */
export const STILL_MS = 5000;
/** a trim's shortest kept part */
export const TRIM_MIN = 100;
/** the clips lane's step (a frame at 24 fps): trims snap to it, I / O count in it */
export const FRAME_MS = 1000 / 24;
/** the view modes, like Blender's viewport shading */
export const MODES = ['shot', 'take', 'rendered', 'generated'] as const;
export type Mode = (typeof MODES)[number];
export type Source = 'still' | 'take' | 'rendered' | 'generated';
/** the lane's words (the page passes them in its language) */
export type Word = Source | 'chosen' | 'review' | 'rejected' | 'audition';

const CHAIN: Record<Mode, Source[]> = { shot: ['still'], take: ['take', 'still'], rendered: ['rendered', 'take', 'still'], generated: ['generated', 'rendered', 'take', 'still'] };

/** a clip the main view plays as a video (the rest is the 3D view) */
export const isVideo = (source: Source | string | undefined) => source === 'rendered' || source === 'generated';

/** a video record as the lane needs it (newest first in its list) */
export interface VideoIn {
	/** the generation's id */
	id: number;
	file: string | null;
	/** its length from the record (ms), null = unknown */
	ms: number | null;
	approved?: boolean;
	take?: string | null;
}
export interface ClipsIn {
	take: TakeIn | null;
	rendered: VideoIn[];
	generated: VideoIn[];
}
/** one shot's clip */
export interface Clip {
	source: Source;
	length: number;
	from: number;
	take: TakeIn | null;
	/** a video of unknown length (it counts STILL_MS until its metadata is read) */
	probe: boolean;
	/** the video's generation id */
	gen?: number;
	file?: string | null;
	approved?: boolean;
	/** the Variants board's current card plays here (#125 – the audition) */
	audition?: boolean;
	/** a generated clip's status bullet: ✓ chosen, ? review, ✕ rejected (the board's marks) */
	status?: 'chosen' | 'review' | 'rejected' | null;
}
/** a shot's override: the board's auditioned card (a video record of the shot) */
export interface Over {
	pick?: VideoIn | null;
}
export interface ClipItem extends TimelineItem {
	length: number;
	clip: Clip;
}
export interface ClipLane extends TimelineLane {
	items: ClipItem[];
}

/** what a generation record says (storyboard-generations.ts Generation) – the fields the lane reads */
export interface GenerationLike {
	id: number;
	shot: string;
	kind: string;
	status: string;
	generator: string;
	take: string | null;
	file: string | null;
	duration: number | null;
	mark: string | null;
	approved: boolean;
}

/** a shot's clips from the shot (its active take) and the chapter's records */
export function clipsInOf(shot: Pick<Shot, 'id' | 'takes'>, generations: readonly GenerationLike[]): ClipsIn {
	const take = shotTake(shot);
	const videos = generations
		.filter((g) => g.shot === shot.id && g.kind === 'video' && g.status === 'succeeded')
		.sort((a, b) => b.id - a.id)
		.map((g) => ({ g, v: { id: g.id, file: g.file, ms: g.duration && g.duration > 0 ? Math.round(g.duration * 1000) : null, approved: g.approved, take: g.take } satisfies VideoIn }));
	return {
		take,
		rendered: take ? videos.filter(({ g }) => g.generator === RENDER_GENERATOR && g.take === take.id).map(({ v }) => v) : [],
		generated: videos.filter(({ g }) => g.generator !== RENDER_GENERATOR && g.mark !== 'rejected').map(({ v }) => v)
	};
}

/** a video's length: the one read from its file (`known`, by generation id), else the record's, else unknown */
const msOf = (v: VideoIn, known: Readonly<Record<number, number>>) => (known[v.id] > 0 ? known[v.id] : v.ms && v.ms > 0 ? v.ms : null);
function video(source: Source, v: VideoIn, known: Readonly<Record<number, number>>, take: TakeIn | null): Clip {
	const ms = msOf(v, known);
	return { source, length: ms ?? STILL_MS, from: 0, take, probe: ms == null, gen: v.id, file: v.file, approved: v.approved };
}

/** one shot's clip in a view mode */
export function clipOf(entry: ClipsIn | undefined, mode: Mode | string, known: Readonly<Record<number, number>> = {}, over: Over | null = null): Clip {
	const take = entry?.take && entry.take.length > 0 ? entry.take : null;
	for (const source of CHAIN[mode as Mode] ?? CHAIN.shot) {
		if (source === 'generated') {
			// the board's current card auditions as the shot's Generated clip (owner 2026-10-20, VARIANTS-CANVAS §3.1 Q3)
			if (over?.pick) return { ...video(source, over.pick, known, take), audition: true };
			const vs = entry?.generated ?? [];
			const g = vs.find((x) => x.approved) ?? vs[0];
			if (g) return video(source, g, known, take);
		} else if (source === 'rendered') {
			const r = take && (entry?.rendered ?? []).find((x) => x.take === take.id);
			if (r) return video(source, r, known, take);
		} else if (source === 'take') {
			if (!take) continue;
			const whole = mode === 'take';
			return { source, length: whole ? take.length : Math.max(1, take.kept), from: whole ? 0 : take.in, take, probe: false };
		}
	}
	return { source: 'still', length: STILL_MS, from: 0, take, probe: false };
}

/** a generation's short name: g3 of "sb-2--g3.webm", else its id (#41) */
export const shortGen = (clip: Pick<Clip, 'gen' | 'file'>) => /--(g\d+)\./.exec(clip.file ?? '')?.[1] ?? `#${clip.gen}`;

/** a clip in words – the item's label (its accessible name, the hover): "sb-2 · take", "sb-2 · generated g3 · chosen" */
export function labelOf(id: string, clip: Clip, words: Partial<Record<Word, string>> = {}): string {
	const w = (k: Word) => words[k] ?? k;
	if (clip.gen === undefined) return `${id} · ${w(clip.source)}`;
	const status = clip.status ?? (clip.approved && clip.source === 'generated' ? 'chosen' : null);
	return [`${id} · ${w(clip.source)} ${shortGen(clip)}`, status ? w(status) : null, clip.audition ? w('audition') : null].filter(Boolean).join(' · ');
}

/** the clips lane: `ids` = the picked shots in toon order, `clips` = each shot's clips, `known` = lengths read from the files */
export function clipsLane(ids: readonly string[], mode: Mode | string, clips: Readonly<Record<string, ClipsIn>>, known: Readonly<Record<number, number>> = {}, words: Partial<Record<Word, string>> = {}, label = 'Clips', over: Readonly<Record<string, Over>> = {}, marks: Readonly<Record<number, 'review' | 'rejected'>> = {}) {
	const items: ClipItem[] = [];
	const probe: number[] = [];
	let at = 0;
	for (const id of ids) {
		const clip = clipOf(clips[id], mode, known, over[id] ?? null);
		if (clip.source === 'generated' && clip.gen !== undefined) clip.status = clip.approved ? 'chosen' : (marks[clip.gen] ?? null);
		if (clip.probe && clip.gen !== undefined) probe.push(clip.gen);
		const item: ClipItem = { id, start: at, length: clip.length, label: labelOf(id, clip, words), state: clip.source, clip };
		if (mode === 'take' && clip.source === 'take' && clip.take) item.trim = { in: clip.take.in, out: clip.take.out, min: TRIM_MIN };
		items.push(item);
		at += clip.length;
	}
	const lane: ClipLane = { id: 'clips', label, items, tall: true, step: FRAME_MS };
	return { lane, span: at, probe };
}

/** the clip under `t`: { item, off } – [start, end); from the end of the last one on, the last one at its end */
export function clipAt(items: readonly ClipItem[], t: number): { item: ClipItem; off: number } | null {
	const last = items.at(-1);
	const item = items.find((i) => t >= i.start && t < i.start + i.length) ?? (last && t >= last.start ? last : null);
	return item ? { item, off: Math.min(t - item.start, item.length) } : null;
}

/** `t` out of a cut part (Play plays the kept parts): before a trim's in → the in, at / past its out → its clip's end
 *  (the next clip's start – and that one's in, when it is cut too) */
export function kept(items: readonly ClipItem[], t: number): number {
	for (let n = 0; n <= items.length; n++) {
		const c = clipAt(items, t);
		const trim = c?.item.trim;
		if (!c || !trim || c.off >= c.item.length) return t;
		if (c.off < trim.in) return c.item.start + trim.in;
		if (c.off < trim.out) return t;
		t = c.item.start + c.item.length;
	}
	return t;
}

// ── The trim (Take mode, research/TIMELINE.md §4 – way C) ─────────────────────────────────────────────────

export type Edge = 'in' | 'out' | 'slip';

/** which edge a trim moved, for a caller that does not say: both = a slip */
export function movedEdge(was: { in: number; out: number }, k: { in: number; out: number }): Edge {
	const [i, o] = [k.in !== was.in, k.out !== was.out];
	return i && o ? 'slip' : i ? 'in' : 'out';
}

/** where a trim puts the playhead: on the in for 'in' and 'slip', on the last kept frame for 'out' */
export const trimPoint = (it: { start: number }, k: { in: number; out: number }, edge: Edge, step = FRAME_MS) => it.start + (edge === 'out' ? Math.max(k.in, k.out - step) : k.in);

/** I / O: the take under `t` trimmed there – I: the frame under `t` is the first one kept, O: the last one kept; the other
 *  edge stays, the kept part ≥ its min and inside the take. null where the clip under `t` has no trim */
export function trimAt(items: readonly ClipItem[], t: number, edge: 'in' | 'out', step = FRAME_MS): { item: ClipItem; in: number; out: number } | null {
	const c = clipAt(items, t);
	const tr = c?.item.trim;
	if (!c || !tr) return null;
	const length = c.item.length;
	const f = Math.floor(Math.min(Math.max(0, c.off), length - 1) / step + 1e-6);
	if (edge === 'in') return { item: c.item, in: Math.max(0, Math.min(f * step, tr.out - tr.min)), out: tr.out };
	return { item: c.item, in: tr.in, out: Math.min(length, Math.max((f + 1) * step, tr.in + tr.min)) };
}

// ── The steps' timelines (workflows/storyboard-studio README "The layout": its table) ─────────────────────

/** a step's timeline the first time: its view mode and whether it is folded */
export const STEP_TIMELINE: Record<string, { mode: Mode; folded: boolean }> = {
	script: { mode: 'shot', folded: true },
	shots: { mode: 'shot', folded: true },
	layout: { mode: 'shot', folded: true },
	handheld: { mode: 'take', folded: false },
	generate: { mode: 'generated', folded: false },
	export: { mode: 'rendered', folded: true }
};
export const isMode = (m: unknown): m is Mode => (MODES as readonly unknown[]).includes(m);
/** the step's remembered timeline (a bad value: the step's first) */
export function stepTimeline(saved: unknown, step: string): { mode: Mode; folded: boolean } {
	const first = STEP_TIMELINE[step] ?? STEP_TIMELINE.shots;
	const s = saved && typeof saved === 'object' ? (saved as Record<string, unknown>) : {};
	return { mode: isMode(s.mode) ? s.mode : first.mode, folded: typeof s.folded === 'boolean' ? s.folded : first.folded };
}

// ── The filmstrip (a video clip's frames one after another – owner 2026-10-20: "show the frame after frame") ────

/** tiles at the video's aspect along a clip w × h (px on screen): how many fit (the last one cut) and each one's time */
export function filmstrip(o: { width: number; height: number; w: number; h: number; length: number; max?: number }): number[] {
	if (!(o.width > 0 && o.height > 0 && o.w > 0 && o.h > 0 && o.length > 0)) return [];
	const tile = (o.h * o.width) / o.height;
	const n = Math.min(o.max ?? 24, Math.max(1, Math.ceil(o.w / tile - 1e-6)));
	// the frame at each tile's left edge (its share of the clip), never past the last frame
	return Array.from({ length: n }, (_, i) => Math.min(o.length - 1, Math.round(((i * tile) / o.w) * o.length)));
}
