// "comfy-local": the existing local ComfyUI pass (scripts/storyboard/comfy.mjs – Qwen-Image-Edit on the owner's GPU)
// behind the generator interface: the job runs `node comfy.mjs storyboards/<toon>/<ch> --only <shot>`, which picks its
// own inputs (the clean render + the figures' character images) and its own prompt, writes
// _out/<ch>/generated/<shot>--g<n>.png and its generated.jsonl record itself; the seed the manager picked goes along
// (`--seed`). Image 1 – the picture edited – is the job's still (VARIANTS-CANVAS §3.6 "Improve on this"): none or the
// render without texts = comfy.mjs's own; a generated image of the shot = `--from <that file>` (comfy.mjs records it as
// the still and its parent); the approved image = `--from` the file it was approved from (originalOf). Any other still
// (the saved frame with its texts, a layout variant's frame, a frame of a video) is refused in the estimate already.
// Free (the own GPU); cancel stops the process. COMFY_URL as for comfy.mjs.

import { execFileSync, spawn } from 'node:child_process';
import { resolve } from 'node:path';

/** a generated image as comfy.mjs's --from takes it (its own check): <shot>--g<n>.png|jpg|webp */
const EDITABLE = /^([\w-]+?)--g\d+\.(png|jpe?g|webp)$/i;

/** originalOf(key) → the generated file name the approved image's key is a copy of, or null (generators/index.mjs) */
export function createComfyLocal({ root, env = process.env, node = process.execPath, script = resolve(root, 'scripts/storyboard/comfy.mjs'), originalOf = () => null } = {}) {
	const url = () => env.COMFY_URL ?? 'http://127.0.0.1:8188';

	/** image 1 for a job: { from: null } = comfy.mjs's own render, { from: '<shot>--g<n>.png' } = edit that result, or
	 *  { error } – why this still can't be image 1 */
	function imageOne(job) {
		const key = String(job.still ?? '');
		if (!key || key === `out:${job.ch}/clean/${job.shot}.png`) return { from: null };
		const own = (file) => (file && EDITABLE.exec(file)?.[1] === job.shot ? file : null);
		const generated = /^out:([a-z0-9-]+)\/generated\/([^/]+)$/.exec(key);
		const chosen = /^boards:dataset\/files\/([a-z0-9-]+)\/([\w-]+)\/chosen\.\w+$/.exec(key);
		const file = generated ? (generated[1] === job.ch ? own(generated[2]) : null) : chosen && chosen[1] === job.ch ? own(originalOf(key)) : null;
		if (file) return { from: file };
		const name = key.replace(/^frame:/, '').replace(/#(\w+)$/, ' ($1 frame)').split('/').pop();
		return { error: `ComfyUI (this PC) edits the render (no texts) or a generated image of ${job.shot} – not ${name}` };
	}

	return {
		id: 'comfy-local',
		label: 'ComfyUI (this PC)',
		transport: 'local',
		kinds: ['image'],
		paid: false,
		note: 'Qwen-Image-Edit on the local ComfyUI (comfy.mjs): the clean render (or a generated image of the shot – an edit) + the character images, photo noir',
		// still = image 1, the picture edited (source: 'edit' – research/GENERATORS.md §2 caps); the prompt is comfy.mjs's own
		models: [{ id: 'qwen-image-edit', label: 'Qwen-Image-Edit (comfy.mjs defaults)', kind: 'image', mode: 'comfy', inputs: ['still'], source: 'edit', seed: true }],
		async status() {
			const ok = await fetch(`${url()}/system_stats`, { signal: AbortSignal.timeout(1500) }).then((r) => r.ok).catch(() => false);
			return ok ? { configured: true } : { configured: false, how: `No ComfyUI at ${url()} – start the ComfyUI app (or set COMFY_URL).` };
		},
		estimate: (job) => {
			const one = imageOne(job);
			return { credits: 0, usd: 0, seconds: null, note: `free – the own GPU (~10 min a shot on the RTX 2070)${one.from ? ` · edits ${one.from}` : ''}`, ...(one.error ? { error: one.error } : {}) };
		},
		run(job, ctx) {
			return new Promise((ok, no) => {
				const one = imageOne(job);
				if (one.error) return no(new Error(one.error));
				const seed = job.seed != null ? ['--seed', String(job.seed)] : [];
				const from = one.from ? ['--from', one.from] : [];
				const child = spawn(node, [script, `storyboards/${job.toon}/${job.ch}`, '--only', job.shot, ...seed, ...from], { cwd: root, env, windowsHide: true });
				let out = '';
				const take = (b) => { out += b; const last = out.trim().split('\n').at(-1); if (last) ctx.progress(null, last.slice(0, 160)); };
				child.stdout.on('data', take);
				child.stderr.on('data', take);
				const stop = () => { try { process.platform === 'win32' ? execFileSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], { stdio: 'ignore' }) : child.kill('SIGKILL'); } catch {} };
				ctx.signal.addEventListener('abort', stop, { once: true });
				child.on('error', no);
				child.on('close', (code) => {
					if (ctx.signal.aborted) return no(Object.assign(new Error('cancelled'), { name: 'AbortError' }));
					const file = new RegExp(`(${job.shot}--g\\d+\\.png) \\(`).exec(out)?.[1];
					if (file) return ok({ written: [file] });
					no(new Error(out.trim().split('\n').at(-1) || `comfy.mjs ended with ${code}`));
				});
			});
		}
	};
}
