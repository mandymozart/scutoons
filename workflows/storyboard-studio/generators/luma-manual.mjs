// "luma-manual" – Luma by hand (owner 2026-10-19: "the luma tab is weird. this should be somehow now be covered by the
// generate right?"): the former Luma panel as a generator, transport 'manual' (research/GENERATORS.md §2). There is no
// API call: the Generate panel shows the shot's full prompt (scripts/storyboard/luma.js via POST /api/prompt –
// everything set for the shot, also unsaved changes) to copy, the render without texts and the references to attach on
// lumalabs.ai. The result comes back through the manual form's result slot (VARIANTS-CANVAS §3.7 "Manual": POST
// /api/generated with the form's `job` – recorded as THIS generator's candidate, model 'manual', with the prompt, the
// still attached – role 'reference', Luma's image reference; job-fields.mjs – and the parent); a drop on the board
// stays a source ('upload'). Free here (the owner's Luma subscription), always configured; the manager never starts a
// job for it, and run() refuses anyway.
// README "Generators", the interface: jobs.mjs.

export function createLumaManual() {
	return {
		id: 'luma-manual',
		label: 'Luma – by hand',
		transport: 'manual',
		kinds: ['image', 'video'],
		paid: false,
		note: 'Done by hand on lumalabs.ai: copy the prompt, attach the render and the references there – then put the result into the result slot here.',
		models: [{ id: 'manual', label: 'By hand on lumalabs.ai', kind: 'image', mode: 'manual', inputs: ['still', 'prompt'] }],
		status: async () => ({ configured: true }),
		estimate: () => ({ credits: 0, usd: 0, seconds: null, note: 'by hand on lumalabs.ai' }),
		run: async () => { throw new Error('Luma – by hand has no API: copy the prompt to lumalabs.ai and put the result into the result slot'); }
	};
}
