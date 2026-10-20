// "Improve on this" · Vary · More – the pure part (#125; the port of #62's improve.js – agents/research/
// VARIANTS-CANVAS.md §3.3, §3.6). A board card FILLS the Generate form (storyboard-generate.svelte.ts) – nothing is
// sent: Generate asks with the cost as always. Here: what a card is as an input (cardOf), which models take it the way
// Improve puts it in (fits), the generator + model an Improve starts with (pickImproveModel), where it goes in the form
// (fieldsFor – the creator's fields: still · control, both generation ids; a still set by hand since stays over the
// card's), the line's words as a code (howOf – the UI says them), why a card can't (refusal), More ▾'s "Use as start
// frame / keyframe" and "Still: a frame" (useAsOf, frameStillOf), Vary's inputs from the record (varyFields) and its
// length (durationFor), the moment a frame is cut at (frameSeconds). Tested in test/storyboard-improve.test.ts.
import type { BoardNode, BoardRecord } from './storyboard-board-layout.ts';
import { shortName, statusOf } from './storyboard-board-layout.ts';
import type { Generation, GeneratorInfo, GeneratorModel } from './storyboard-generations.ts';

/** a card as an input */
export interface ImproveCard {
	/** its words ("g2"; a source: '' – the UI names the render / the take) */
	name: string;
	/** its generation (null: the render, a take) */
	id: number | null;
	as: 'image' | 'video' | 'render' | 'take';
	video: boolean;
	prompt: string;
	seed: number | null;
	/** Vary's sibling hangs there */
	parent: number | null;
	take: string | null;
}

/** what fills the form: an Improve on a card or a Vary of one */
export type ImproveFrom = ImproveCard & {
	mode: 'improve' | 'vary';
	shot: string;
	/** a Vary: the card's own inputs; an Improve: unused */
	fields?: FormFields;
	/** a still set by hand since (More ▾ "Use as start frame", a frame, the panel's select) – over the card's */
	still?: number | null;
	/** the card's seed goes again (else New: the provider picks one) */
	useSeed: boolean;
	/** the generator was switched (its own takes no such card) – the one before, for ✕ */
	switched: { from: string; to: string } | null;
	prev: { generator: string; model: string } | null;
};

/** the form fields Improve sets (generation ids; null = none) */
export interface FormFields {
	still?: number | null;
	control?: number | null;
	characters?: string[];
}

type Rec = Pick<BoardRecord, 'id' | 'kind' | 'parents' | 'generator'> & Partial<Pick<Generation, 'file' | 'prompt' | 'seed' | 'model' | 'duration' | 'inputs'>> & Pick<BoardRecord, 'approved' | 'mark'>;

/** a board card as an input – null when nothing can go in */
export function cardOf(node: Pick<BoardNode, 'kind' | 'source' | 'take'> & { record: Rec | null } | null | undefined): ImproveCard | null {
	if (!node) return null;
	const r = node.record;
	if (r) {
		const video = r.kind === 'video';
		return { name: shortName(r), id: r.id, as: video ? 'video' : 'image', video, prompt: r.prompt ?? '', seed: r.seed ?? null, parent: r.parents?.[0] ?? null, take: null };
	}
	if (node.source === 'render') return { name: '', id: null, as: 'render', video: false, prompt: '', seed: null, parent: null, take: null };
	if (node.source === 'take' && node.take) return { name: node.take, id: null, as: 'take', video: false, prompt: '', seed: null, parent: null, take: node.take };
	return null;
}

const roles = (m: GeneratorModel | null | undefined) => m?.refs.roles ?? [];
const required = (m: GeneratorModel | null | undefined) => m?.refs.required ?? [];
export const takesStill = (m: GeneratorModel | null | undefined) => roles(m).includes('still');
export const takesControl = (m: GeneratorModel | null | undefined) => roles(m).includes('control');
const isManual = (m: GeneratorModel | null | undefined) => m?.mode === 'manual';

/** the model takes the card the way Improve puts it in: an image → a model with a still (the first frame, the image to
 *  edit, by hand the reference – not one that needs a video); a video → a video to video model (its control); the
 *  render, a take and a Vary → any */
export function fits(from: Pick<ImproveFrom, 'mode' | 'as'> | null | undefined, m: GeneratorModel | null | undefined): boolean {
	if (!m) return false;
	if (from?.mode !== 'improve') return true;
	if (from.as === 'image') return takesStill(m) && !required(m).includes('control');
	if (from.as === 'video') return takesControl(m);
	return true;
}

type Choice = Pick<GeneratorInfo, 'id' | 'configured' | 'unavailable' | 'models'>;
/** the generator + model an Improve starts with: the current ones if the model fits, else the current generator's first
 *  that fits, else the first generator with one – one that can run first, one that makes it before one by hand */
export function pickImproveModel(generators: readonly Choice[], cur: { generator: string; model: string } | null, from: Pick<ImproveFrom, 'mode' | 'as'>): { generator: string; model: string; switched: boolean } | null {
	const ok = (g: Choice | undefined) => (g?.models ?? []).filter((m) => fits(from, m));
	const own = ok(generators.find((g) => g.id === cur?.generator));
	if (own.length && cur) return { generator: cur.generator, model: (own.find((m) => m.id === cur.model) ?? own[0]).id, switched: false };
	const rank = (g: Choice, m: GeneratorModel) => (g.configured && !g.unavailable ? 0 : 2) + (isManual(m) ? 1 : 0);
	let best: { generator: string; model: string; rank: number } | null = null;
	for (const g of generators) for (const m of ok(g)) if (!best || rank(g, m) < best.rank) best = { generator: g.id, model: m.id, rank: rank(g, m) };
	return best ? { generator: best.generator, model: best.model, switched: true } : null;
}

/** where the card goes in the form for model m (only the fields it sets): an image → the still; a video → the control
 *  (+ the shot's chosen image as the keyframe, if the model takes a still); the render / a take → nothing (the shot's own
 *  inputs); Vary → the card's own inputs; a still set by hand since → over the card's */
export function fieldsFor(from: ImproveFrom | null | undefined, m: GeneratorModel | null | undefined, chosen: number | null = null): FormFields {
	if (!from || !m) return {};
	const own = ownFields(from, m, chosen);
	return from.still !== undefined && takesStill(m) ? { ...own, still: from.still } : own;
}
function ownFields(from: ImproveFrom, m: GeneratorModel, chosen: number | null): FormFields {
	if (from.mode === 'vary') return { still: takesStill(m) ? (from.fields?.still ?? null) : null, control: takesControl(m) ? (from.fields?.control ?? null) : null, characters: from.fields?.characters ?? [] };
	if (from.as === 'image') return takesStill(m) ? { still: from.id } : {};
	if (from.as === 'video') return takesControl(m) ? { control: from.id, ...(takesStill(m) ? { still: chosen } : {}) } : {};
	return {};
}

/** how the card goes in for model m – a code the UI words (storyboard.improve_how_*):
 *  start · edit · reference · keyframe (an image) · video_key · video_nokey (a video) · own (the render, a take) · vary ·
 *  set (a still set here over the card's) · none (this model takes no such card) */
export type HowCode = 'start' | 'edit' | 'reference' | 'video_key' | 'video_nokey' | 'own' | 'vary' | 'set' | 'none';
export function howOf(from: ImproveFrom | null | undefined, m: GeneratorModel | null | undefined, chosen: number | null = null): HowCode | null {
	if (!from || !m) return null;
	if (from.mode === 'vary') return from.still !== undefined && takesStill(m) ? 'set' : 'vary';
	if (from.as === 'render' || from.as === 'take') return from.still !== undefined && takesStill(m) ? 'set' : 'own';
	if (!fits(from, m)) return 'none';
	if (from.still !== undefined && takesStill(m) && from.as === 'image') return 'set';
	if (from.as === 'video') return takesStill(m) && (from.still !== undefined ? from.still : chosen) !== null ? 'video_key' : 'video_nokey';
	if (isManual(m)) return 'reference';
	return m.mode === 'image_to_video' ? 'start' : 'edit';
}

/** why a card can't be improved / varied / used – a code (storyboard.improve_no_*), '' = it can */
export type Refusal = '' | 'rejected' | 'running' | 'source' | 'dropped' | 'generator' | 'model' | 'take';
export function refusal(node: Pick<BoardNode, 'kind' | 'source'> & { record: Rec | null } | null | undefined, what: 'improve' | 'vary' | 'use', generators: readonly Choice[] = []): Refusal {
	if (!node || node.kind === 'running') return 'running';
	const r = node.record;
	if (statusOf(r) === 'rejected') return 'rejected';
	if (what === 'use' && !r) return 'source';
	if (what !== 'vary') return '';
	if (!r) return 'source';
	if (!r.generator || r.generator === 'upload' || r.generator === 'frame') return 'dropped';
	const g = generators.find((x) => x.id === r.generator);
	if (generators.length && !g) return 'generator';
	if (g && !g.models.some((m) => m.id === r.model)) return 'model';
	return '';
}

/** Vary's inputs: the record's references as the form's fields – a work-storage file → the shot's generation that names
 *  it, a character image (its url) → its key in the form's list (`characters`: { key, url }) */
export function varyFields(inputs: Generation['inputs'], gens: readonly Pick<Generation, 'id' | 'work_path'>[], characters: readonly { key: string; url: string }[] = []): FormFields {
	const out: FormFields = { still: null, control: null, characters: [] };
	for (const i of inputs ?? []) {
		const g = i.work ? gens.find((x) => x.work_path === i.work) : undefined;
		if ((i.role === 'still' || i.role === 'control') && g && out[i.role] === null) out[i.role] = g.id;
		if (i.role === 'character' && i.url) {
			const c = characters.find((x) => x.url === i.url);
			if (c) out.characters!.push(c.key);
		}
	}
	return out;
}

/** the generation ids an Improve's job names as its parents: the form's references, the main input first (the control
 *  video before the still – the layout hangs a result under its first parent) */
export const parentsOf = (fields: { control?: number | null; still?: number | null }): number[] => [...new Set([fields.control, fields.still].filter((x): x is number => typeof x === 'number'))];

/** a Vary's length: the card's (its record's duration), within what the model takes – null when it takes none */
export function durationFor(m: GeneratorModel | null | undefined, seconds: number | null | undefined): number | null {
	if (!m?.durations || seconds == null || !Number.isFinite(Number(seconds))) return null;
	const { min, max } = m.durations;
	return Math.min(max, Math.max(min, Math.round(Number(seconds))));
}

/** More ▾ "Use as start frame / keyframe" on an image card (it ADDS the card – the still – the rest stays): offered only
 *  where the form's model takes it as one (image → video: the start frame; video → video: the keyframe at 0) */
export function useAsOf(m: GeneratorModel | null | undefined): 'start' | 'keyframe' | null {
	if (!takesStill(m)) return null;
	if (m!.mode === 'image_to_video') return 'start';
	return m!.mode === 'video_to_video' ? 'keyframe' : null;
}

/** More ▾ "Still: a frame" on a video card: the form's model takes a still */
export const frameStillOf = (m: GeneratorModel | null | undefined): boolean => takesStill(m);

/** the moment a frame is cut at (seconds): the first, the last (a hair before the end – a decoder shows nothing past
 *  it), or a time in ms within the length */
export function frameSeconds(frame: 'first' | 'last' | number, duration: number | null | undefined): number {
	const len = Number.isFinite(duration) && (duration as number) > 0 ? (duration as number) : 0;
	if (frame === 'first') return 0;
	if (frame === 'last') return Math.max(0, len - 0.05);
	const s = Math.max(0, Number(frame) / 1000 || 0);
	return len ? Math.min(s, Math.max(0, len - 0.05)) : s;
}

/** a seed goes with the job only for a model that takes one (Runway's – params.seed; Luma ignores it) */
export const takesSeed = (m: GeneratorModel | null | undefined) => !!m && m.kind === 'video' && !isManual(m);
