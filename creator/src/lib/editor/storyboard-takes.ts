// A shot's handheld takes (#126; research/HANDHELD-CAMERA.md, workflows/storyboard-studio README "Handheld camera"): they
// live INLINE in the shot (`Shot.takes`, STUDIO-IN-CREATOR §2.2) – the camera on per shot, the takes with their raw
// samples, smoothing, scale and trim (in / out, s), one active. Pure: every change is a function on the shot that the
// page runs inside one document edit (an undo step, autosave). Also the record a rendered take leaves in the ledger.
import type { Shot, ShotTakes, Take, TakeSample } from '@scutoons/content/storyboard';
import { packSample } from './storyboard-handheld.ts';

/** a take needs this many samples (a phone that sent nothing → no take) */
export const TAKE_MIN_SAMPLES = 2;
/** samples are stored at most this often (Hz) – the phone sends ~60 Hz; the document stays small (a 10 s take ≈ 30 KB) */
export const STORE_HZ = 30;
/** the rendered take's generator in the ledger (storyboard_generations.generator) */
export const RENDER_GENERATOR = 'handheld-render';

export const takesOf = (shot: Pick<Shot, 'takes'> | null | undefined): ShotTakes => shot?.takes ?? { on: false, active: null, takes: [] };
export const activeTake = (doc: ShotTakes | null | undefined): Take | null => doc?.takes.find((t) => t.id === doc.active) ?? null;
/** the take's whole length (s) */
export const takeEnd = (take: Pick<Take, 'samples'>): number => take.samples.at(-1)?.t ?? 0;

/** the next take id: t1, t2 … after the highest one there (a deleted one's number is not reused while a later exists) */
export function nextTakeId(takes: readonly Pick<Take, 'id'>[]): string {
	return `t${takes.reduce((m, t) => Math.max(m, Number(t.id.slice(1)) || 0), 0) + 1}`;
}

/** the samples as stored: rounded, at most STORE_HZ, t from 0 (the last one always kept – the take's end) */
export function thinSamples(samples: readonly TakeSample[], hz = STORE_HZ): TakeSample[] {
	if (!samples.length) return [];
	const t0 = samples[0].t,
		gap = 1 / hz - 1e-6;
	const out: TakeSample[] = [];
	let last = -Infinity;
	samples.forEach((s, i) => {
		const t = s.t - t0;
		if (t - last >= gap || i === samples.length - 1) {
			out.push(packSample({ ...s, t }));
			last = t;
		}
	});
	return out;
}

/** a recorded take (null: too short) */
export function newTake(takes: readonly Pick<Take, 'id'>[], samples: readonly TakeSample[], opts: { smooth: number; scale: number; now?: string }): Take | null {
	const kept = thinSamples(samples);
	if (kept.length < TAKE_MIN_SAMPLES) return null;
	return { id: nextTakeId(takes), created: opts.now ?? new Date().toISOString(), smooth: opts.smooth, scale: opts.scale, in: 0, out: takeEnd({ samples: kept }), samples: kept };
}

// ── Changes of a shot (run inside the page's document edit) ─────────────────────────────────────────────

const doc = (shot: Shot): ShotTakes => (shot.takes ??= { on: false, active: null, takes: [] });

export const setOn = (on: boolean) => (shot: Shot) => void (doc(shot).on = on);
export const addTake = (take: Take) => (shot: Shot) => {
	const d = doc(shot);
	d.takes = [...d.takes, take];
	d.active = take.id;
};
export const setActive = (id: string) => (shot: Shot) => {
	const d = doc(shot);
	if (d.takes.some((t) => t.id === id)) d.active = id;
};
/** the take goes; the active one → the newest left */
export const removeTake = (id: string) => (shot: Shot) => {
	const d = doc(shot);
	d.takes = d.takes.filter((t) => t.id !== id);
	if (d.active === id) d.active = d.takes.at(-1)?.id ?? null;
};
/** a field of one take (smooth, scale) */
export const setTakeField = (id: string, field: 'smooth' | 'scale', value: number) => (shot: Shot) => {
	const t = doc(shot).takes.find((x) => x.id === id);
	if (t) t[field] = value;
};
/** the kept part (ms of the take, the timeline's) → in / out in s, clamped to the take */
export const setTrim = (id: string, inMs: number, outMs: number) => (shot: Shot) => {
	const t = doc(shot).takes.find((x) => x.id === id);
	if (!t) return;
	const end = takeEnd(t);
	const tin = Math.min(Math.max(0, inMs / 1000), end);
	t.in = +tin.toFixed(3);
	t.out = +Math.min(end, Math.max(tin, outMs / 1000)).toFixed(3);
};

/** a take on the timeline (ms): its in / out (a missing in = 0, a missing out = its end), its length, the kept part */
export interface TakeIn {
	id: string;
	in: number;
	out: number;
	length: number;
	kept: number;
}
export function takeIn(take: Take | null | undefined): TakeIn | null {
	if (!take) return null;
	const length = Math.round(takeEnd(take) * 1000);
	if (!(length > 0)) return null;
	const tin = Math.min(length, Math.max(0, Math.round((Number(take.in) || 0) * 1000)));
	const out = Number.isFinite(Number(take.out)) ? Math.round(Number(take.out) * 1000) : length;
	const tout = Math.max(tin, Math.min(length, out));
	return { id: take.id, in: tin, out: tout, length, kept: tout - tin };
}
/** the shot's active take on the timeline – only with the camera on */
export const shotTake = (shot: Pick<Shot, 'takes'> | null | undefined): TakeIn | null => (shot?.takes?.on ? takeIn(activeTake(shot.takes)) : null);

// ── A rendered take in the ledger ───────────────────────────────────────────────────────────────────────

/** the record of a take rendered into the workspace folder (POST /storyboard/generations, kind video) */
export function renderRecord(o: { toon: string; chapter: number; shot: string; take: Take; file: string; hash: string; machine: string; fps: number; seconds: number; ratio?: string | null }) {
	return {
		toon: o.toon,
		chapter: o.chapter,
		shot: o.shot,
		kind: 'video',
		generator: RENDER_GENERATOR,
		mode: 'beauty',
		take: o.take.id,
		file: o.file,
		file_hash: o.hash,
		machine: o.machine,
		duration: +o.seconds.toFixed(3),
		...(o.ratio && /^[1-9]\d{0,3}:[1-9]\d{0,3}$/.test(o.ratio) ? { ratio: o.ratio } : {}),
		params: { fps: o.fps, smooth: o.take.smooth, scale: o.take.scale, in: o.take.in, out: o.take.out }
	};
}

/** the sha-256 of a file (hex) – the record names the bytes it means */
export async function sha256(blob: Blob): Promise<string> {
	const digest = await crypto.subtle.digest('SHA-256', await blob.arrayBuffer());
	return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, '0')).join('');
}
