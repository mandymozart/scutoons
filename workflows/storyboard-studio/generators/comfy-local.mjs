// "comfy-local": the existing local ComfyUI pass (scripts/storyboard/comfy.mjs – Qwen-Image-Edit on the owner's GPU)
// behind the generator interface, unchanged: the job runs `node comfy.mjs storyboards/<toon>/<ch> --only <shot>`, which
// picks its own inputs (the clean render + the figures' character images), writes _out/<ch>/generated/<shot>--g<n>.png
// and its generated.jsonl line itself. Free (the own GPU); cancel stops the process. COMFY_URL as for comfy.mjs.

import { execFileSync, spawn } from 'node:child_process';
import { resolve } from 'node:path';

export function createComfyLocal({ root, env = process.env, node = process.execPath, script = resolve(root, 'scripts/storyboard/comfy.mjs') } = {}) {
	const url = () => env.COMFY_URL ?? 'http://127.0.0.1:8188';
	return {
		id: 'comfy-local',
		label: 'ComfyUI (this PC)',
		transport: 'local',
		kinds: ['image'],
		paid: false,
		note: 'Qwen-Image-Edit on the local ComfyUI (comfy.mjs): the clean render + the character images, photo noir',
		models: [{ id: 'qwen-image-edit', label: 'Qwen-Image-Edit (comfy.mjs defaults)', kind: 'image', mode: 'comfy', inputs: [] }],
		async status() {
			const ok = await fetch(`${url()}/system_stats`, { signal: AbortSignal.timeout(1500) }).then((r) => r.ok).catch(() => false);
			return ok ? { configured: true } : { configured: false, how: `No ComfyUI at ${url()} – start the ComfyUI app (or set COMFY_URL).` };
		},
		estimate: () => ({ credits: 0, usd: 0, seconds: null, note: 'free – the own GPU (~10 min a shot on the RTX 2070)' }),
		run(job, ctx) {
			return new Promise((ok, no) => {
				const child = spawn(node, [script, `storyboards/${job.toon}/${job.ch}`, '--only', job.shot], { cwd: root, env, windowsHide: true });
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
