// Two more generators: the documented stub for the cloud ComfyUI, and the fake one for tests.

/** "comfyui-remote": the same ComfyUI graphs on a cloud GPU – a RunPod Serverless endpoint or Comfy Cloud
 *  (agents/research/STORYBOARD.md §3 "Open models on a serverless GPU"). Not built: when it comes, it takes the graph of
 *  comfy.mjs (graph()), uploads the inputs, POSTs the job to the endpoint (RUNPOD: /run + /status/{id}, cancel
 *  /cancel/{id}; key COMFY_REMOTE_KEY in the repo's .env), books the GPU seconds as its cost – the interface is the
 *  same as comfy-local's, so the studio needs no change. */
export function createComfyRemote() {
	return {
		id: 'comfyui-remote',
		label: 'ComfyUI (cloud GPU)',
		transport: 'api',
		kinds: ['image', 'video'],
		paid: true,
		note: 'RunPod Serverless / Comfy Cloud – not built yet',
		models: [],
		status: async () => ({ configured: false, how: 'Not built yet – the cloud ComfyUI endpoint comes later (STORYBOARD §3).' }),
		estimate: () => ({ credits: 0, usd: 0, seconds: null, error: 'not built yet' }),
		run: async () => { throw new Error('comfyui-remote is not built yet'); }
	};
}

/** "fake": for tests and the studio check (STUDIO_FAKE_GENERATOR=1) – priced like a paid generator (1 credit per
 *  second, fake credits), so the confirmation and the spend counter run; after a few progress steps it returns the
 *  still (or a 1 × 1 PNG) as the result. */
const PIXEL = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGNgYGD4DwABBAEAwS2OUAAAAABJRU5ErkJggg==', 'base64');
export function createFake({ read = null, stepMs = 300, steps = 5, fail = false } = {}) {
	return {
		id: 'fake',
		label: 'Fake (test)',
		transport: 'api',
		kinds: ['image'],
		paid: true,
		creditUsd: 0.01,
		note: 'a stand-in for tests – fake credits',
		models: [{ id: 'fake-1', label: 'Fake – returns the still', kind: 'image', mode: 'fake', inputs: ['still', 'prompt', 'duration'], durations: { min: 1, max: 10 }, price: { creditsPerSecond: 1 }, seed: true }],
		status: async () => ({ configured: true }),
		estimate: (job) => { const s = Math.max(1, Math.min(10, Math.round(Number(job.duration) || 5))); return { credits: s, usd: s / 100, seconds: s, note: 'fake credits' }; },
		async run(job, ctx) {
			ctx.remote(`fake-${Date.now()}`);
			for (let i = 1; i <= steps; i++) {
				await new Promise((ok, no) => {
					if (ctx.signal.aborted) return no(Object.assign(new Error('cancelled'), { name: 'AbortError' }));
					const t = setTimeout(ok, stepMs);
					ctx.signal.addEventListener('abort', () => { clearTimeout(t); no(Object.assign(new Error('cancelled'), { name: 'AbortError' })); }, { once: true });
				});
				ctx.progress(i / steps, `step ${i} of ${steps}`);
			}
			if (fail) throw new Error('the fake failed on purpose');
			ctx.cost(Math.max(1, Math.min(10, Math.round(Number(job.duration) || 5))));
			return { files: [{ bytes: job.stillFile && read ? read(job.stillFile) : PIXEL, ext: 'png' }], meta: { fake: true } };
		}
	};
}
