// The Generate panel's state (README "Generators", generators/jobs.mjs): the generators and whether they are
// configured, the shot's inputs (pre-filled), "Build from the shot" (the shot's full prompt + the references to attach),
// the jobs of the shot (polled while one runs) and the session's spend. What is sent (the job, the estimate, start with
// the cost confirm): generate-send.js. Every price is shown in euro (owner 2026-10-20: "preise in euro") at the rate
// the server serves (generators/rate.mjs); the provider's own figures (credits, USD) stay in the tooltip. What is sent
// and kept stays in USD (`confirm`, the jobs, the spend).
// The form belongs to ONE shot of one chapter (`inputsFor` = "<ch>/<shot>", `loaded`): another shot or chapter clears it
// before its inputs load – and it stays cleared when they can't (nothing of the one before can go out, VARIANTS-CANVAS
// §3.6 – the server refuses it too).
// FILLED FROM A CARD (VARIANTS-CANVAS §3.6, #62 stage 3 – the board's Improve on this · Vary, improve.svelte.js): `from` =
// the card, put where its kind fits the model (improve.js fieldsFor), its prompt (editable – the new record keeps the
// edit), the seed New or the card's, a Vary its length; the pickers keep only the models that take it; nothing is sent;
// ✕ (clearFrom) = the shot's own inputs. A still set by hand or from More ▾ while a card fills the form stays over the
// card's (`from.still` – the line names it). The pre-filled still follows the model (`auto`).
import { api, ch, fail, notify, studio, toon } from './studio.svelte.js';
import { chosenImage, defaultStill, durationFor, fieldsFor, fits, pickModel, stillLabel, switchWhy, takesStill } from './improve.js';

const remember = (k, v) => { try { localStorage.setItem(`storyboard-studio:${k}`, v); } catch {} };
const recall = (k) => { try { return localStorage.getItem(`storyboard-studio:${k}`); } catch { return null; } };

export const gen = $state({
	generators: [],
	spend: { usd: 0, credits: 0, jobs: 0 },
	/** the euro rate: { eurPerUsd, usdPerEur, date, source: 'ecb' | 'fallback' | 'env' } (GET /api/generators, …/jobs) */
	rate: null,
	/** the shot's inputs: { stills, controls, characters, style, prompt, aspect, duration } – of `inputsFor` ("<ch>/<shot>"
	 *  – null while another shot's load) */
	inputs: null,
	inputsFor: null,
	shot: null,
	/** seed: null = a new one (the server picks it), else the card's; characters / style: a Vary's own (sent where the
	 *  model takes them) */
	form: { generator: '', model: '', still: '', control: '', prompt: '', duration: 5, seed: null, characters: [], style: [] },
	/** the still is the pre-fill – it follows the model; false once picked by hand or set from a card */
	auto: true,
	/** inputs the shot's lists don't name (a frame at a time, a Vary's own) – options of their selects too */
	extra: { stills: [], controls: [] },
	/** frames cut for a thumbnail (POST /api/generate/frame): { [frame key]: url } */
	frames: {},
	/** the card the form was filled from (Improve on / Vary – improve.svelte.js): { mode: 'improve' | 'vary', ch, shot,
	 *  name, id, as, key, url, video, prompt, seed, useSeed, parent, fields + duration (Vary), generator / model (Vary),
	 *  still (one set by hand over the card's – null = the card's), prev (the generator + model before the first fill),
	 *  switched, applied } – null = the shot's own inputs */
	from: null,
	/** counts the fills from the board: App opens and unfolds the Generate panel */
	reveal: 0,
	/** "Build from the shot": { shot, text, refs } – the shot's full prompt as built + the references to attach (null = not
	 * built); building = running */
	built: null,
	building: false,
	estimate: null,
	jobs: [],
	busy: false
});

export const current = () => gen.generators.find((g) => g.id === gen.form.generator) ?? null;
export const model = () => current()?.models.find((m) => m.id === gen.form.model) ?? current()?.models[0] ?? null;
export const wants = (input) => !!model()?.inputs.includes(input);
/** the form holds the inputs of this shot (default: the form's) of the current chapter */
export const loaded = (id = gen.shot) => !!id && gen.inputsFor === `${ch()}/${id}`;

export async function loadGenerators() {
	const r = await api('GET', '/api/generators');
	gen.generators = r.generators;
	gen.spend = r.spend;
	gen.rate = r.rate;
	const want = recall('generator');
	const pick = r.generators.find((g) => g.id === want) ?? r.generators.find((g) => g.configured && g.paid) ?? r.generators.find((g) => g.configured) ?? r.generators[0];
	if (pick && !gen.form.generator) setGenerator(pick.id);
}

/** where the form's shot is: { ch, shot } (improve.js) */
export const at = () => ({ ch: ch(), shot: gen.shot });
/** the stills / videos the form offers: the shot's + the extra ones */
export const stills = () => [...(gen.inputs?.stills ?? []), ...gen.extra.stills];
export const controls = () => [...(gen.inputs?.controls ?? []), ...gen.extra.controls];
/** the models the pickers offer: while a card fills the form, only those that take it (improve.js fits) */
export const fitting = (g) => (g?.models ?? []).filter((m) => fits(gen.from, m, at()));
/** where the card goes for the model now (its still set by hand over it) */
const cardFields = (from = gen.from) => (from?.applied ? fieldsFor(from, model(), { chosen: chosenImage(stills()), at: at() }) : {});

/** a key the shot's lists don't name becomes an extra option (its select would show "(?)" else) */
function addExtra({ still, control } = {}) {
	if (still && !stills().some((s) => s.key === still)) gen.extra = { ...gen.extra, stills: [...gen.extra.stills, { key: still, label: stillLabel(still, gen.shot), url: null }] };
	if (control && !controls().some((s) => s.key === control)) gen.extra = { ...gen.extra, controls: [...gen.extra.controls, { key: control, label: stillLabel(control, gen.shot), url: null }] };
}
/** the still + video for the model now: a card goes where the model takes it; a pre-filled still (or one the model
 *  can't take) becomes the model's default */
function follow() {
	const m = model();
	const f = cardFields();
	if ('still' in f) gen.auto = false;
	else if (gen.auto || (gen.form.still && !takesStill(m, gen.form.still, at()))) prefill();
	Object.assign(gen.form, f);
	addExtra(f);
}
/** the model's default still – it follows the model from now on */
function prefill() {
	gen.auto = true;
	gen.form.still = defaultStill(model(), stills(), at());
}

export function setGenerator(id, keep = true) {
	gen.form.generator = id;
	if (keep) remember('generator', id);
	const g = current();
	const ok = fitting(g);
	const want = recall(`model:${id}`);
	if (!ok.some((m) => m.id === gen.form.model)) gen.form.model = (ok.find((m) => m.id === want) ?? ok[0])?.id ?? '';
	follow();
}
export function setModel(id) {
	gen.form.model = id;
	remember(`model:${gen.form.generator}`, id);
	follow();
}
/** a still picked by hand (the Still select) or set from a card (More ▾ – it adds the card, the rest stays); while a card
 *  fills the form it stays over the card's own (`from.still`), back to the card's when that one is picked again */
export function setStill(key, { reveal = false } = {}) {
	const k = key ?? '';
	gen.form.still = k;
	gen.auto = false;
	if (gen.from?.applied) gen.from.still = k === cardFields({ ...gen.from, still: null }).still ? null : k;
	addExtra({ still: k });
	if (reveal) gen.reveal++;
}
/** the seed: the card's (`own`) or a new one */
export function setSeed(own) {
	if (!gen.from) return;
	gen.from.useSeed = own;
	gen.form.seed = own ? gen.from.seed : null;
}

/** the shot's own inputs (the prompt, the length, the first take's pass, the model's default still, a new seed) */
function reset() {
	const r = gen.inputs;
	Object.assign(gen.form, { prompt: r?.prompt ?? '', duration: r?.duration ?? 5, control: r?.controls[0]?.key ?? '', seed: null, characters: [], style: [] });
	gen.extra = { stills: [], controls: [] };
	prefill();
}
/** the generator + model before the board's first fill, if they are still there */
function restore(prev) {
	const g = prev && gen.generators.find((x) => x.id === prev.generator);
	if (g) Object.assign(gen.form, { generator: g.id, model: g.models.some((m) => m.id === prev.model) ? prev.model : g.models[0]?.id ?? '' });
}
/** ✕ on the line: back to the shot's own inputs (and the generator before the fill) – nothing of the card stays */
export function clearFrom() {
	const f = gen.from;
	gen.from = null;
	restore(f?.prev);
	reset();
}

/** the card into the form (once – a reload of the lists keeps what was edited since): the generator + model that take
 *  it (Vary: its own), the shot's defaults under it, then the card's fields, its prompt, the seed (Vary: its length) */
function applyFrom() {
	const f = gen.from;
	const pick = f.mode === 'vary' ? { generator: f.generator, model: f.model, switched: false } : pickModel(gen.generators, gen.form, f, at());
	if (!pick) {
		gen.from = null;
		return notify(`No generator takes ${f.name} – nothing filled in.`, 'warning');
	}
	const before = current()?.label ?? gen.form.generator;
	f.prev ??= { generator: gen.form.generator, model: gen.form.model };
	f.switched = pick.switched ? { to: pick.generator, why: switchWhy(before, f) } : null;
	Object.assign(gen.form, { generator: pick.generator, model: pick.model });
	reset();
	f.applied = true;
	f.still = null;
	gen.form.prompt = f.prompt || gen.inputs?.prompt || '';
	gen.form.seed = f.useSeed ? f.seed : null;
	const d = f.mode === 'vary' ? durationFor(model(), f.duration) : null;
	if (d != null) gen.form.duration = d;
	// the card's fields (the render and a take keep the pre-filled still – it follows the model)
	follow();
	gen.reveal++;
}

let loading = 0;
/** the shot's inputs → the form, pre-filled (the still: the model's default; the newest take's pass); another shot or
 *  chapter clears the form first (and a card of the one before goes – the generator before it comes back); a card
 *  waiting for this shot goes in after (applyFrom) */
export async function loadInputs(id) {
	const n = ++loading;
	const key = `${ch()}/${id}`;
	gen.shot = id;
	if (gen.inputsFor !== key) {
		if (gen.from && (gen.from.shot !== id || gen.from.ch !== ch())) {
			restore(gen.from.prev);
			gen.from = null;
		}
		Object.assign(gen, { inputsFor: null, inputs: null, built: null, estimate: null, extra: { stills: [], controls: [] } });
		Object.assign(gen.form, { still: '', control: '', prompt: '', seed: null, characters: [], style: [] });
	}
	if (!gen.generators.length) await loadGenerators();
	const r = await api('GET', `/api/generate/inputs?toon=${toon()}&ch=${ch()}&id=${id}`);
	if (n !== loading || key !== `${ch()}/${gen.shot}`) return;
	const same = gen.inputsFor === key;
	gen.inputs = r;
	gen.inputsFor = key;
	const has = (list, k) => list.some((x) => x.key === k);
	if (!same) reset();
	else {
		// a still that is gone → the default; "none" picked (or set by a card) stays none
		if ((gen.form.still || gen.auto) && !has(stills(), gen.form.still)) prefill();
		if (!has(controls(), gen.form.control)) gen.form.control = r.controls[0]?.key ?? '';
	}
	if (gen.from?.shot === id && !gen.from.applied) applyFrom();
	await loadJobs();
}

/** a frame's thumbnail (the Still select's picture): cut once (POST /api/generate/frame – cached on disk) */
const cutting = new Set();
export function frameUrl(key) {
	if (!String(key ?? '').startsWith('frame:') || cutting.has(key)) return;
	cutting.add(key);
	api('POST', '/api/generate/frame', { key }).then((r) => { gen.frames = { ...gen.frames, [key]: `${r.url}?t=${r.mtime}` }; }, (e) => { cutting.delete(key); fail(e); });
}

/** "Build from the shot" (every generator with a prompt): the shot as edited right now, unsaved changes too, as one
 * prompt – scripts/storyboard/luma.js via POST /api/prompt – + the references to attach. `fill`: into the generator's
 * Prompt, replacing it (the button says so); Luma – by hand only shows and copies `gen.built`, so a prompt typed for
 * another generator stays as it was */
export async function buildPrompt({ fill = false } = {}) {
	const id = studio.sel?.id;
	if (!id) return;
	gen.building = true;
	try {
		const r = await api('POST', '/api/prompt', { toon: toon(), ch: ch(), shot: $state.snapshot(studio.sel.shot) });
		if (studio.sel?.id !== id) return;
		gen.built = { shot: id, text: r.text, refs: r.refs };
		if (fill) gen.form.prompt = r.text;
	} finally {
		gen.building = false;
	}
}

/** by hand (Luma): the prompt to copy and to record with the result – the built one, or the card's while a card fills
 *  the form (editable then) */
export const manualPrompt = () => (gen.from ? gen.form.prompt : gen.built?.shot === gen.shot ? gen.built.text : '');

const EUR = new Intl.NumberFormat('de-DE', { style: 'currency', currency: 'EUR' });
const EUR_FINE = new Intl.NumberFormat('de-DE', { style: 'currency', currency: 'EUR', maximumFractionDigits: 4 });
/** a USD amount in euro at the served rate: "0,22 €" (`fine`: up to 4 decimals – "0,0089 €" for a credit; an amount
 *  under half a cent gets them too, so a cost never shows as "0,00 €") */
export function euro(usd, fine = false) {
	if (!gen.rate) return '–';
	const eur = Number(usd) * gen.rate.eurPerUsd;
	return (fine || (eur !== 0 && Math.abs(eur) < 0.005) ? EUR_FINE : EUR).format(eur);
}
/** the provider's figures behind a euro amount, for its tooltip: "25 credits = 0.25 USD · 1 USD = 0.8909 € (ECB 2026-10-19)" */
export function usdTitle({ credits, usd } = {}) {
	const r = gen.rate;
	const figure = `${credits != null ? `${credits} credit${credits === 1 ? '' : 's'} = ` : ''}${Number(usd ?? 0).toFixed(2)} USD`;
	const rate = !r ? '' : r.source === 'env' ? `1 USD = ${r.eurPerUsd} € (STUDIO_EUR_PER_USD)` : r.source === 'fallback' ? `1 USD ≈ ${r.eurPerUsd.toFixed(4)} € (the fixed fallback, ECB ${r.date} – no newer rate could be fetched)` : `1 USD = ${r.eurPerUsd.toFixed(4)} € (ECB ${r.date})`;
	return rate ? `${figure} · ${rate}` : figure;
}

let poll = null;
export async function loadJobs() {
	if (!gen.shot) return;
	const before = new Map(gen.jobs.map((j) => [j.id, j.status]));
	const r = await api('GET', `/api/generate/jobs?ch=${ch()}&id=${gen.shot}`);
	gen.jobs = r.jobs;
	gen.spend = r.spend;
	if (r.rate) gen.rate = r.rate;
	const finished = r.jobs.filter((j) => j.status === 'succeeded' && before.get(j.id) && before.get(j.id) !== 'succeeded');
	if (finished.length) { studio.version++; notify(`Generated: ${finished.flatMap((j) => j.files).join(', ')} – on the board (Variants, V).`, 'success'); }
	const active = r.jobs.some((j) => j.status === 'queued' || j.status === 'running');
	clearTimeout(poll);
	if (active) poll = setTimeout(() => loadJobs().catch(() => {}), 1500);
}
