// "luma-manual" – Luma by hand (owner 2026-10-19: "the luma tab is weird. this should be somehow now be covered by the
// generate right?"): the former Luma panel as a generator, transport 'manual' (research/GENERATORS.md §2). There is no
// API call: the Generate panel shows the shot's full prompt (scripts/storyboard/luma.js via POST /api/prompt –
// everything set for the shot, also unsaved changes) to copy, the render without texts and the references to attach on
// lumalabs.ai; the result comes back by dropping it onto Variants (POST /api/generated). Free here (the owner's Luma
// subscription), always configured; the manager never starts a job for it, and run() refuses anyway.
// README "Generators", the interface: jobs.mjs.

export function createLumaManual() {
	return {
		id: 'luma-manual',
		label: 'Luma – by hand',
		transport: 'manual',
		kinds: ['image', 'video'],
		paid: false,
		note: 'Done by hand on lumalabs.ai: copy the prompt, attach the render and the references there – drop the result onto Variants.',
		models: [{ id: 'manual', label: 'By hand on lumalabs.ai', kind: 'image', mode: 'manual', inputs: ['prompt'] }],
		status: async () => ({ configured: true }),
		estimate: () => ({ credits: 0, usd: 0, seconds: null, note: 'by hand on lumalabs.ai' }),
		run: async () => { throw new Error('Luma – by hand has no API: copy the prompt to lumalabs.ai and drop the result onto Variants'); }
	};
}
