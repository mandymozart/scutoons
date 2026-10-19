// The Generate panel's state (README "Generators", generators/jobs.mjs): the generators and whether they are
// configured, the shot's inputs (pre-filled), the estimate shown before anything is sent, the jobs of the shot (polled
// while one runs) and the session's spend. A paid job is sent only after confirmDialog with the estimate.
import { confirmDialog } from '@scutoons/ui';
import { api, ch, studio, toon } from './studio.svelte.js';

const remember = (k, v) => { try { localStorage.setItem(`storyboard-studio:${k}`, v); } catch {} };
const recall = (k) => { try { return localStorage.getItem(`storyboard-studio:${k}`); } catch { return null; } };

export const gen = $state({
	generators: [],
	spend: { usd: 0, credits: 0, jobs: 0 },
	/** the shot's inputs: { stills, controls, characters, style, prompt, aspect, duration } */
	inputs: null,
	shot: null,
	form: { generator: '', model: '', still: '', control: '', prompt: '', duration: 5 },
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
	if (!same || !has(r.stills, gen.form.still)) gen.form.still = r.stills[0]?.key ?? '';
	if (!same || !has(r.controls, gen.form.control)) gen.form.control = r.controls[0]?.key ?? '';
	await loadJobs();
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

const money = (usd) => `${usd.toFixed(2)} USD`;
export const costText = (e) => (!e ? '' : e.error ? e.error : !e.paid ? `Free${e.note ? ` – ${e.note}` : ''}` : `≈ ${e.credits} credits · ${money(e.usd)}${e.seconds ? ` · ${Number(e.seconds.toFixed(1))} s` : ''}${e.ratio ? ` · ${e.ratio}` : ''}`);

export async function start() {
	const g = current();
	const e = await api('POST', '/api/generate/estimate', { job: job() });
	gen.estimate = e;
	if (e.error) { studio.message = e.error; return; }
	if (e.paid) {
		const ok = await confirmDialog({
			title: `Send to ${g.label}?`,
			message: `${e.model} · ${costText(e).replace(/^≈ /, 'about ')}${e.rate ? ` (${e.rate})` : ''}. Charged to your ${g.label} account – this session so far: ${money(gen.spend.usd)}.`,
			confirmLabel: `Generate for ${money(e.usd)}`
		});
		if (!ok) return;
	}
	gen.busy = true;
	try {
		await api('POST', '/api/generate', { job: job(), confirm: e.usd });
		await loadJobs();
	} catch (err) {
		studio.message = err.message;
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
	const finished = r.jobs.filter((j) => j.status === 'succeeded' && before.get(j.id) && before.get(j.id) !== 'succeeded');
	if (finished.length) { studio.version++; studio.message = `Generated: ${finished.flatMap((j) => j.files).join(', ')} – in Variants.`; }
	const active = r.jobs.some((j) => j.status === 'queued' || j.status === 'running');
	clearTimeout(poll);
	if (active) poll = setTimeout(() => loadJobs().catch(() => {}), 1500);
}
