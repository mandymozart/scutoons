// The Generate panel's state (README "Generators", generators/jobs.mjs): the generators and whether they are
// configured, the shot's inputs (pre-filled), "Build from the shot" (the shot's full prompt + the references to attach),
// the estimate shown before anything is sent, the jobs of the shot (polled while one runs) and the session's spend. A
// paid job is sent only after confirmDialog with the estimate. Every price is shown in euro (owner 2026-10-20: "preise
// in euro") at the rate the server serves (generators/rate.mjs); the provider's own figures (credits, USD) stay in the
// tooltip. What is sent and kept stays in USD (`confirm`, the jobs, the spend).
import { confirmDialog } from '@scutoons/ui';
import { api, ch, fail, notify, studio, toon } from './studio.svelte.js';

const remember = (k, v) => { try { localStorage.setItem(`storyboard-studio:${k}`, v); } catch {} };
const recall = (k) => { try { return localStorage.getItem(`storyboard-studio:${k}`); } catch { return null; } };

export const gen = $state({
	generators: [],
	spend: { usd: 0, credits: 0, jobs: 0 },
	/** the euro rate: { eurPerUsd, usdPerEur, date, source: 'ecb' | 'fallback' | 'env' } (GET /api/generators, …/jobs) */
	rate: null,
	/** the shot's inputs: { stills, controls, characters, style, prompt, aspect, duration } */
	inputs: null,
	shot: null,
	form: { generator: '', model: '', still: '', control: '', prompt: '', duration: 5 },
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

export async function loadGenerators() {
	const r = await api('GET', '/api/generators');
	gen.generators = r.generators;
	gen.spend = r.spend;
	gen.rate = r.rate;
	const want = recall('generator');
	const pick = r.generators.find((g) => g.id === want) ?? r.generators.find((g) => g.configured && g.paid) ?? r.generators.find((g) => g.configured) ?? r.generators[0];
	if (pick && !gen.form.generator) setGenerator(pick.id);
}

export function setGenerator(id) {
	gen.form.generator = id;
	remember('generator', id);
	const g = current();
	if (!g?.models.some((m) => m.id === gen.form.model)) gen.form.model = recall(`model:${id}`) && g?.models.some((m) => m.id === recall(`model:${id}`)) ? recall(`model:${id}`) : g?.models[0]?.id ?? '';
}
export function setModel(id) {
	gen.form.model = id;
	remember(`model:${gen.form.generator}`, id);
}

/** the shot's inputs → the form, pre-filled (the first still: the approved image, else the render; the newest take's pass) */
export async function loadInputs(id) {
	const same = gen.shot === id && gen.inputs;
	gen.shot = id;
	const r = await api('GET', `/api/generate/inputs?toon=${toon()}&ch=${ch()}&id=${id}`);
	if (gen.shot !== id) return;
	gen.inputs = r;
	const has = (list, key) => list.some((x) => x.key === key);
	if (!same) Object.assign(gen.form, { prompt: r.prompt, duration: r.duration });
	if (!same) gen.built = null;
	if (!same || !has(r.stills, gen.form.still)) gen.form.still = r.stills[0]?.key ?? '';
	if (!same || !has(r.controls, gen.form.control)) gen.form.control = r.controls[0]?.key ?? '';
	await loadJobs();
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

const job = () => ({ generator: gen.form.generator, model: gen.form.model, toon: toon(), ch: ch(), shot: gen.shot, aspect: gen.inputs?.aspect,
	prompt: wants('prompt') ? gen.form.prompt : '', duration: wants('duration') ? Number(gen.form.duration) : undefined,
	still: wants('still') ? gen.form.still || null : null, control: wants('control') ? gen.form.control || null : null });

let timer = null;
export function estimate() {
	clearTimeout(timer);
	timer = setTimeout(async () => {
		if (!gen.shot || !gen.form.generator) return;
		try { gen.estimate = await api('POST', '/api/generate/estimate', { job: job() }); } catch (e) { gen.estimate = { error: e.message }; }
	}, 150);
}

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
const extras = (e) => `${e.seconds ? ` · ${Number(e.seconds.toFixed(1))} s` : ''}${e.ratio ? ` · ${e.ratio}` : ''}`;
export const costText = (e) => (!e ? '' : e.error ? e.error : !e.paid ? `Free${e.note ? ` – ${e.note}` : ''}` : `≈ ${euro(e.usd)}${extras(e)}`);
/** the estimate line's tooltip: the provider's figures */
export const costTitle = (e) => (e?.paid && !e.error ? usdTitle(e) : '');

export async function start() {
	const g = current();
	const e = await api('POST', '/api/generate/estimate', { job: job() });
	gen.estimate = e;
	if (e.error) { notify(e.error, 'error'); return; }
	if (e.paid) {
		const ok = await confirmDialog({
			title: `Send to ${g.label}?`,
			message: `${e.model} · about ${euro(e.usd)}${extras(e)} – ${e.credits} credits${e.rate ? ` (${e.rate})` : ''}. Charged to your ${g.label} account – this session so far: ${euro(gen.spend.usd)}.`,
			confirmLabel: `Generate for ${euro(e.usd)}`
		});
		if (!ok) return;
	}
	gen.busy = true;
	try {
		await api('POST', '/api/generate', { job: job(), confirm: e.usd });
		await loadJobs();
	} catch (err) {
		fail(err);
	} finally {
		gen.busy = false;
	}
}

export async function cancel(jobId) {
	await api('POST', '/api/generate/cancel', { job: jobId });
	await loadJobs();
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
