// What the Generate panel sends (README "Generators", generators/jobs.mjs): the job built from the form, the estimate
// shown before anything is sent (euro – the provider's credits + USD in the tooltip), start – a paid job only after
// confirmDialog with the estimate –, cancel. The form and its state: generate.svelte.js. A job goes out only for the
// shot whose inputs the form holds (`loaded` – a form left from another shot or chapter never does, VARIANTS-CANVAS
// §3.6; the server refuses it too: generators/index.mjs jobOf), and the body that was estimated and confirmed is the
// body that is sent.
import { confirmDialog } from '@scutoons/ui';
import { api, ch, fail, notify, studio, toon } from './studio.svelte.js';
import { current, euro, gen, loadJobs, loaded, usdTitle, wants } from './generate.svelte.js';

/** the form as a job (only what the model takes) */
export const job = () => ({ generator: gen.form.generator, model: gen.form.model, toon: toon(), ch: ch(), shot: gen.shot, aspect: gen.inputs?.aspect,
	prompt: wants('prompt') ? gen.form.prompt : '', duration: wants('duration') ? Number(gen.form.duration) : undefined, seed: gen.form.seed ?? null,
	still: wants('still') ? gen.form.still || null : null, control: wants('control') ? gen.form.control || null : null,
	...(wants('characters') && gen.form.characters.length ? { characters: gen.form.characters } : {}), ...(wants('style') && gen.form.style.length ? { style: gen.form.style } : {}) });

let timer = null;
/** the estimate follows the form (none while the shot's inputs load) */
export function estimate() {
	clearTimeout(timer);
	timer = setTimeout(async () => {
		if (!gen.shot || !gen.form.generator || !loaded()) return void (gen.estimate = null);
		try { gen.estimate = await api('POST', '/api/generate/estimate', { job: job() }); } catch (e) { gen.estimate = { error: e.message }; }
	}, 150);
}

const extras = (e) => `${e.seconds ? ` · ${Number(e.seconds.toFixed(1))} s` : ''}${e.ratio ? ` · ${e.ratio}` : ''}`;
export const costText = (e) => (!e ? '' : e.error ? e.error : !e.paid ? `Free${e.note ? ` – ${e.note}` : ''}` : `≈ ${euro(e.usd)}${extras(e)}`);
/** the estimate line's tooltip: the provider's figures */
export const costTitle = (e) => (e?.paid && !e.error ? usdTitle(e) : '');

/** Generate: the estimate again, a paid one asks with its cost, then the same body goes out */
export async function start() {
	const g = current();
	if (!loaded() || gen.shot !== studio.sel?.id) return notify(`${studio.sel?.id ?? 'The shot'}'s inputs are not loaded yet – nothing is sent.`, 'error');
	const body = job();
	let e;
	try { e = await api('POST', '/api/generate/estimate', { job: body }); } catch (err) { return fail(err); }
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
		await api('POST', '/api/generate', { job: body, confirm: e.usd });
		await loadJobs();
	} catch (err) {
		fail(err);
	} finally {
		gen.busy = false;
	}
}

export const cancel = (jobId) => api('POST', '/api/generate/cancel', { job: jobId }).then(loadJobs);
