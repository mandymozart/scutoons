// "Improve on this" · Vary · More – the pure part (agents/research/VARIANTS-CANVAS.md §3.3, §3.6, §3.7; issue #62 stage
// 3; test/improve-form.test.mjs). A board card fills the Generate panel – nothing is sent (Generate asks with the cost as
// always). Here: what a card is as an input (cardOf), which models take it the way Improve puts it in (fits – the
// panel's pickers keep only those while an Improve is on), where it goes in the form for a model (fieldsFor – today's
// fields: still · control – a still set by hand since over the card's: setStillOf), the generator + model an Improve
// starts with (pickModel), the line's words (howText), which stills a model takes (takesStill – ComfyUI edits only the
// render or a generated image of the shot, comfy-local.mjs), the pre-fill's still (defaultStill – an edit model's and
// the by-hand one's is the render, or none), More ▾'s "Use as start frame / keyframe" and "Still: … frame" (useAsOf,
// frameStillOf), the words for a still the shot's list doesn't name (stillLabel), Vary's chosen image pinned to the
// file it was then (pinChosen) and its length (durationFor). The keys and roles: generators/job-fields.mjs (the
// server's own rules). The state: generate.svelte.js, improve.svelte.js.
import { frameKey, parseFrameKey } from '../../generators/job-fields.mjs';
import { shortId } from './board-layout.js';

const esc = (s) => String(s).replace(/[^\w-]/g, '\\$&');
const CHOSEN_IMAGE = /^boards:dataset\/files\/[a-z0-9-]+\/[\w-]+\/chosen\.(png|jpe?g|webp)$/i;
/** "/out/s1c3/handheld/x--t1--beauty.mp4?v=2" → "out:s1c3/handheld/x--t1--beauty.mp4" (a take's render as a key) */
const keyOfUrl = (url) => { const m = /^\/out\/([^?#]+)/.exec(String(url ?? '')); return m ? `out:${decodeURIComponent(m[1])}` : null; };
const takes = (m, what) => !!m?.inputs?.includes(what);

/** a key ComfyUI's Qwen-Edit takes as image 1 (comfy-local.mjs imageOne): the render without texts, a generated image
 *  of the shot, its chosen image (it edits the file that was approved) – at = { ch, shot } */
export function editable(key, { ch, shot } = {}) {
	const k = String(key ?? '');
	return k === `out:${ch}/clean/${shot}.png`
		|| new RegExp(`^out:${esc(ch)}/generated/${esc(shot)}--g\\d+\\.(png|jpe?g|webp)$`, 'i').test(k)
		|| new RegExp(`^boards:dataset/files/${esc(ch)}/${esc(shot)}/chosen\\.(png|jpe?g|webp)$`, 'i').test(k);
}

/** the model takes this still: none without a still input; an edit model only what it can edit; else any (a frame too) */
export const takesStill = (m, key, at) => takes(m, 'still') && (m.source !== 'edit' || editable(key, at));

/** the pre-fill's still for a model (the panel pre-fills from the shot): an edit model's (ComfyUI) and the one done by
 *  hand (Luma's reference – its panel says "attach the render") is the render – or none: never an earlier result
 *  (an edit of one is picked by hand or comes with Improve on that card; without a render comfy.mjs's own pass says
 *  so); else the first one offered that it takes (the chosen image, else the render); '' = none */
export function defaultStill(m, stills, at) {
	const list = (stills ?? []).filter((s) => takesStill(m, s.key, at));
	if (m?.source === 'edit' || m?.mode === 'manual') return list.find((s) => s.key === `out:${at?.ch}/clean/${at?.shot}.png`)?.key ?? '';
	return list[0]?.key ?? '';
}

/** the shot's chosen image among the offered stills (Improve on a video: the keyframe at 0) – null = none */
export const chosenImage = (stills) => (stills ?? []).find((s) => CHOSEN_IMAGE.test(s.key))?.key ?? null;

/** a board card as an input → { name, id (its record's; null for the render, a layout variant, a take), as ('image' |
 *  'video' | 'render' | 'take' – how Improve puts it in), key, url, video, prompt, seed, parent (Vary's sibling hangs
 *  there), record } – null when nothing can go in. A dropped file goes in as what it is (an image / a video); the
 *  render as the shot's own inputs; a take as its handheld render – `take` = its render ({ url }, Board's takes) */
export function cardOf(node, { ch, shot, take } = {}) {
	const r = node?.record;
	const base = { id: null, prompt: '', seed: null, parent: null, record: null, video: false };
	if (r) {
		const key = r.key ?? (r.file ? `out:${ch}/generated/${r.file}` : null);
		if (!key) return null;
		const video = r.kind === 'video';
		return { ...base, name: shortId(r.id), id: r.id, as: video ? 'video' : 'image', key, url: r.url ?? null, video, prompt: r.prompt ?? '', seed: r.seed ?? null, parent: r.parents?.[0] ?? null, record: r };
	}
	if (node?.source === 'render') return { ...base, name: 'the render', as: 'render', key: `out:${ch}/clean/${shot}.png`, url: `/out/${ch}/clean/${shot}.png` };
	if (node?.source === 'variant' && node.variant?.id) {
		const file = `${shot}--${node.variant.id}.png`;
		return { ...base, name: node.variant.id, as: 'image', key: `out:${ch}/variants/${file}`, url: `/out/${ch}/variants/${file}` };
	}
	const key = node?.source === 'take' ? keyOfUrl(take?.url) : null;
	return key ? { ...base, name: `take ${node.take}`, as: 'take', key, url: take.url, video: true } : null;
}

/** the model takes the card the way Improve puts it in (§3.6 – the panel's pickers keep only these while it is on): an
 *  image → a model whose still is the first frame, an edit's image 1 (only what it can edit) or by hand (the reference);
 *  a video → a video to video model only (its control – a frame of it as a still is More ▾'s "Still: …"); the render,
 *  a take and a Vary → any */
export function fits(from, m, at) {
	if (!m) return false;
	if (from?.mode !== 'improve') return true;
	if (from.as === 'image') return !!m.startFrame || m.mode === 'manual' || (m.source === 'edit' && editable(from.key, at));
	if (from.as === 'video') return takes(m, 'control');
	return true;
}

/** the generator + model an Improve starts with: the current ones if the model fits, else the current generator's first
 *  that fits, else the first generator with a model that fits – a configured one first, one that makes it before one
 *  done by hand (Luma by hand takes anything as its reference) → { generator, model, switched } – null when none takes
 *  it. cur = { generator, model } (the form) */
export function pickModel(generators, cur, from, at) {
	const ok = (g) => (g?.models ?? []).filter((m) => fits(from, m, at));
	const own = ok(generators.find((g) => g.id === cur?.generator));
	if (own.length) return { generator: cur.generator, model: (own.find((m) => m.id === cur.model) ?? own[0]).id, switched: false };
	const rank = (g, m) => (g.configured ? 0 : 2) + (m.mode === 'manual' ? 1 : 0);
	let best = null;
	for (const g of generators) for (const m of ok(g)) if (!best || rank(g, m) < best.rank) best = { generator: g.id, model: m.id, rank: rank(g, m) };
	return best ? { generator: best.generator, model: best.model, switched: true } : null;
}

/** a still set by hand (or from More ▾) over the card's while it fills the form – when model m takes it; else null */
export const setStillOf = (from, m, at) => (from?.still != null && takes(m, 'still') && (from.still === '' || takesStill(m, from.still, at)) ? from.still : null);

/** where the card goes in the form for model m (§3.6, today's fields) – only the fields it sets: an image → the still; a
 *  video → the control (+ the chosen image as the keyframe at 0, if there is one), a still model its first frame; a
 *  take → the control (its render); the render → nothing (the shot's own inputs); Vary → the card's own inputs
 *  (from.fields = job-fields.mjs fieldsOf); a still set by hand since (from.still) over the card's. opts.chosen = the
 *  shot's chosen image's key, opts.at = { ch, shot } */
export function fieldsFor(from, m, { chosen = null, at = null } = {}) {
	if (!from || !m) return {};
	const set = setStillOf(from, m, at);
	return set == null ? ownFields(from, m, chosen) : { ...ownFields(from, m, chosen), still: set };
}
function ownFields(from, m, chosen) {
	if (from.mode === 'vary') return { still: from.fields?.still ?? '', control: from.fields?.control ?? '', characters: from.fields?.characters ?? [], style: from.fields?.style ?? [] };
	if (from.as === 'image') return takes(m, 'still') ? { still: from.key } : {};
	if (from.as === 'video') {
		if (takes(m, 'control')) return { control: from.key, ...(takes(m, 'still') ? { still: chosen ?? '' } : {}) };
		return takes(m, 'still') ? { still: frameKey(from.key, 'first') } : {};
	}
	if (from.as === 'take') return takes(m, 'control') ? { control: from.key } : {};
	return {};
}

/** what model m's still is, in the line's words */
const stillAs = (m) => (m.source === 'edit' ? 'image 1' : m.mode === 'video_to_video' ? 'the keyframe at 0' : m.mode === 'manual' ? 'the reference' : 'the first frame');

/** the line's words under "Improve on g2" / "Vary g2": how the card goes in for model m – and a still set by hand over
 *  the card's (opts.label(key) = its words; the card itself is then no input where it was the still) */
export function howText(from, m, { chosen = null, at = null, label = (k) => k } = {}) {
	if (!from || !m) return '';
	const n = from.name;
	const set = setStillOf(from, m, at);
	const mine = set == null ? '' : set ? `${label(set)} as ${stillAs(m)}` : `no ${stillAs(m).replace(/^the /, '')}`;
	if (from.mode === 'vary') {
		const as = from.unpinned ? ' (the chosen image as it is NOW – its record doesn\'t say which file it was then)' : '';
		return set == null ? `its own inputs${as}, prompt and model – a sibling in its row` : `its prompt and model – ${mine}, set here instead of its own`;
	}
	if (from.as === 'image') {
		if (set != null) return `${mine}, set here – ${n} is not in the form now`;
		return m.source === 'edit' ? `${n} as image 1 – an edit` : m.mode === 'manual' ? `${n} as the reference to attach` : `${n} as the first frame`;
	}
	if (from.as === 'video') {
		if (takes(m, 'control')) return `${n} as the video${set != null ? ` – ${mine}, set here` : takes(m, 'still') ? (chosen ? ' – the chosen image as the keyframe at 0' : ' – no keyframe (no chosen image)') : ''}`;
		return set != null ? `${mine}, set here – ${n} is not in the form now` : `${n}'s first frame as ${stillAs(m)}`;
	}
	const base = from.as === 'take' && takes(m, 'control') ? `${n}'s render as the video` : 'the shot\'s own inputs';
	return set != null ? `${base} – ${mine}, set here` : base;
}

/** a Vary's length: the card's (its record's duration), within what the model takes – null when it takes none */
export function durationFor(m, seconds) {
	if (!takes(m, 'duration') || seconds == null || !Number.isFinite(Number(seconds))) return null;
	const { min = 1, max = 10 } = m.durations ?? {};
	return Math.min(max, Math.max(min, Math.round(Number(seconds))));
}

const GENERATED = /^out:[a-z0-9-]+\/generated\/([^/]+)\.[a-z0-9]+$/i;
/** Vary's inputs: the chosen image (an alias – a later approval changes its file) pinned to the file it stood for when
 *  the card was made – its parent in the record (the parents keep the original's id in input order after the generated
 *  inputs' own: generated-log.mjs parentsOf), as "out:<ch>/generated/<that record's file>" → { inputs, gone (its file
 *  is not there any more – left out), unpinned (no parent known: the alias stays – today's chosen image) }.
 *  records = the shot's records ({ id, file, missing }) */
export function pinChosen(inputs, parents, records, ch) {
	const made = new Set((inputs ?? []).map((i) => GENERATED.exec(String(i?.key ?? ''))?.[1]).filter(Boolean));
	const left = (parents ?? []).filter((p) => !made.has(p));
	const out = { inputs: [], gone: [], unpinned: false };
	for (const i of inputs ?? []) {
		if (!CHOSEN_IMAGE.test(String(i?.key ?? ''))) { out.inputs.push(i); continue; }
		const id = left.shift();
		const r = id && (records ?? []).find((x) => x.id === id);
		if (!id) { out.unpinned = true; out.inputs.push(i); }
		else if (!r?.file || r.missing) out.gone.push(shortId(id));
		else out.inputs.push({ ...i, key: `out:${ch}/generated/${r.file}` });
	}
	return out;
}

/** why the generator was switched (the line says so): "ComfyUI (this PC) takes no video" */
export const switchWhy = (label, from) => `${label} takes no ${from?.as === 'video' ? 'video' : `still like ${from?.name ?? 'this one'}`}`;

/** More ▾ "Use as start frame / keyframe" on an image card (§3.6: it ADDS the card to the form – the still – instead of
 *  replacing it): what the form's model makes of it → { label, ok, why } – offered only where the model takes it
 *  (gen4_turbo / gen4.5: the first frame; Aleph 2: ONE keyframe at 0 today – the adapter's ≤ 5 are GENERATORS §6 step 5) */
export function useAsOf(m, label = 'The model') {
	if (m?.startFrame) return { label: 'Use as start frame', ok: true, why: '' };
	if (m?.keyframes?.max) return { label: 'Use as keyframe (at 0)', ok: true, why: '' };
	return { label: 'Use as start frame / keyframe', ok: false, why: `${label} takes no start frame or keyframe – pick an image to video model (Gen-4) or Aleph 2 in Generate` };
}

/** More ▾ "Still: … frame" on a video card (§3.7 frame → still): whether the form's model takes a frame as its still */
export function frameStillOf(m, label = 'The model') {
	if (!takes(m, 'still')) return { ok: false, why: `${label} takes no still` };
	if (m.source === 'edit') return { ok: false, why: `${label} edits the render or a generated image – not a frame of a video` };
	return { ok: true, why: '' };
}

/** the words for a still / video the shot's lists don't name (a frame at a time, a Vary's own input) – as the server
 *  names its own ("Generated g4.webm – the first frame") */
export function stillLabel(key, shot) {
	const named = (k) => { const b = String(k).split('/').pop(); return b.startsWith(`${shot}--`) ? `Generated ${b.slice(shot.length + 2)}` : b; };
	const f = parseFrameKey(key);
	if (!f) return named(key);
	return `${named(f.key)} – ${typeof f.frame === 'number' ? `the frame at ${(f.frame / 1000).toFixed(1)} s` : `the ${f.frame} frame`}`;
}
