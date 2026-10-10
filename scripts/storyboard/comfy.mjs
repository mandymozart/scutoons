// The generated pass on the owner's own GPU (owner 2026-10-10: "is there not free generator available?", "can we run
// something on my rtx2070 locally", "comfy ui is installed"): the local ComfyUI with Qwen-Image-Edit 2509 + the
// Lightning 4-step LoRA – up to three input images: 1 = our render without texts (the layout: composition, camera,
// poses, the hard light), 2–3 = the figures' character images (a style picture only with --style: Qwen copies its
// content). Free; the rehearsal of the cloud-GPU chain (agents/STORYBOARD.md §9).
//
// On the RTX 2070 (8 GB) + 64 GB RAM (night 2026-10-10): Qwen is ~20 GB + a 9 GB text encoder, so ~16 GB stay in RAM
// and stream to the card each step; fp8 is cast to fp32 (no fp8 on Turing). Fast enough only on a FRESH server per
// image started with --disable-pinned-memory --disable-mmap --cache-none --disable-smart-memory (~105 s a step,
// ~10 min a shot): a long-running server (the desktop app) grows to 53 GB, Windows pages, and a step takes 350 s.
// The per-shot server loop: agents/STORYBOARD.md §9 / BACKLOG (the night run).
//
// The result lands where the studio's uploads go: _out/<ch>/generated/<id>--g<n>.png (Variants, Approve as chosen,
// figma-chapter.mjs), logged in _out/<ch>/generated.jsonl.
//
//   node scripts/storyboard/comfy.mjs storyboards/catpope/s1c3 --only sb-25-4,sb-28-3 [--steps 4] [--seed n] [--mp 1] [--dry]
//   node scripts/storyboard/comfy.mjs storyboards/catpope/s1c3 --missing --fresh [--until 08:30]   the night run: every
//        shot without an image yet, each on its own fresh headless ComfyUI (port 8190), no new shot after --until
//   COMFY_URL=http://127.0.0.1:8188 (default – the ComfyUI app, moved off the viewer's 8000)

import { execFileSync, spawn } from 'node:child_process';
import { openSync } from 'node:fs';
import { appendFileSync, existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { ROOT } from './lib.mjs';
import { STYLE_DIR, shotCharacters, styleFiles } from './refs.mjs';
import { loadStoryboards } from './storyboard.mjs';

// keys from the repo's .env (LUMA_AGENTS_API_KEY) – a variable already set in the shell wins
try { process.loadEnvFile(resolve(ROOT, '.env')); } catch {}

const args = process.argv.slice(2);
const take = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : d; };
const flag = (f) => { const i = args.indexOf(f); return i >= 0 ? (args.splice(i, 1), true) : false; };
const only = take('--only', null)?.split(',');
const steps = Number(take('--steps', 4));
// full quality on a big cloud GPU (2026-10-10, Runpod): COMFY_LORA=none --steps 40 --cfg 4 – no Lightning shortcut
const cfg = Number(take('--cfg', 1));
// how the reference images enter the model – 2511 wants 'index_timestep_zero' (ComfyUI's 2511 template); off = the node's default
const refMethod = take('--ref-method', null);
// the same shots, prompt and images through Luma's Agents API (owner 2026-10-10: 'can we also try the luma api in
// comparison?'): --engine luma [--luma-model uni-1|uni-1-max] [--luma-style manga], key in LUMA_AGENTS_API_KEY
// (platform.lumalabs.ai – billed per image, separate from the Luma subscription)
const engine = take('--engine', 'comfy');
const lumaModel = take('--luma-model', 'uni-1');
const lumaStyle = take('--luma-style', 'auto');
const mp = Number(take('--mp', 1));
const seedArg = take('--seed', null);
const missing = flag('--missing');
const fresh = flag('--fresh');
const until = take('--until', null);
const dry = flag('--dry');
// a style picture as an input image: off by default – Qwen copies its CONTENT (sb-25-1/2 became Cat Pope with books)
const useStyle = flag('--style');
let COMFY = process.env.COMFY_URL ?? 'http://127.0.0.1:8188';
const M = {
	unet: process.env.COMFY_UNET ?? 'qwen_image_edit_2509_fp8_e4m3fn.safetensors',
	clip: process.env.COMFY_CLIP ?? 'qwen_2.5_vl_7b_fp8_scaled.safetensors',
	vae: process.env.COMFY_VAE ?? 'qwen_image_vae.safetensors',
	lora: process.env.COMFY_LORA ?? (steps <= 4 ? 'Qwen-Image-Edit-2509-Lightning-4steps-V1.0-bf16.safetensors' : 'Qwen-Image-Edit-2509-Lightning-8steps-V1.0-bf16.safetensors')
};

const src = args[0];
if (!src) { console.error('usage: comfy.mjs <storyboard folder> --only <ids> [--steps 4|8] [--seed n] [--mp 1] [--dry]'); process.exit(2); }
const ch = basename(resolve(src));
const OUT = resolve(ROOT, 'storyboards/_out', ch);
const gen = resolve(OUT, 'generated');

// PLF in the chapters is the human priest OR the sphynx cat – Cat Pope (the cat-headed pope) only where the story shows
// him, the mirror at the end of chapter 5 (owner 2026-10-10)
const WHO = { 'catpope/human': 'PLF, a human priest in a long white papal robe – a human face, no cat features at all', 'catpope/regular': 'Cat Pope: a priest in a white papal robe with the head of a hairless sphynx cat', 'catpope/cat': 'a hairless sphynx cat – an animal on four legs', 'catpope/mountaintiger': 'a huge white tiger with blue runes' };
/** how much of the picture a figure fills (% of its height) and where – from the storyboard's x / y / h */
function placeOf(shot, fig) {
	const [a, b] = (shot.aspect ?? '9:16').split(':').map(Number);
	const close = ['cu', 'ecu', 'mcu'].includes(fig.framing);
	const pct = close ? 80 : Math.round(((fig.h ?? 0.3) * 844 * 100) / ((390 * b) / a));
	const x = fig.x ?? 0.5, y = fig.y ?? 0.5;
	const where = `${y < 0.34 ? 'top' : y > 0.66 ? 'bottom' : 'middle'}${x < 0.34 ? ' left' : x > 0.66 ? ' right' : ''}`;
	return { pct, where, small: !close && pct < 25 };
}
/** the scene heading + place: outdoors in the snow or inside */
function setting(sb, shot) {
	const heading = sb.source?.heading ?? '';
	const place = (sb.locations ?? []).find((l) => l.ref === shot.location);
	const name = place ? place.place ?? place.new ?? place.ref : '';
	const ext = /\bEXT\b/.test(heading);
	const snow = ext && /winter|schnee|snow|berg|wald|lichtung|pfad|weg/i.test(`${heading} ${name}`);
	return `Setting: ${name || heading}${place?.time ? `, ${place.time}` : ''}. ${ext ? `Outdoors${snow ? ' in winter – the ground is deep snow, not floor tiles or stone' : ''}.` : 'Indoors.'}`;
}
/** Qwen-Edit wants a short, direct instruction – not the long Luma brief. A small figure gets no character image
 *  (the model then copies the reference big into the frame – sb-25-1, 2026-10-10) – it is described instead. */
function instruction(sb, shot, chars) {
	const light = { ...(sb.light ?? {}), ...(shot.light ?? {}) };
	let n = 1;
	const who = chars.map((c) => {
		const name = WHO[`${c.cast.character}/${c.cast.persona ?? 'regular'}`] ?? c.cast.character;
		const size = c.place.small ? `a SMALL figure, only about ${c.place.pct}% of the picture's height, at the ${c.place.where} – keep it that small` : `about ${c.place.pct}% of the picture's height, at the ${c.place.where}`;
		const ref = c.withImage ? ` – looks exactly like the character in image ${++n}${c.image?.mood ? `, ${c.image.mood} expression` : ''}` : '';
		return `the ${c.fig.pose ?? 'standing'} figure is ${name}: ${size}${ref}`;
	});
	return [
		'Redraw image 1 as a finished black-and-white webtoon storyboard frame: clean, crisp ink line art on white paper, solid black shadows, high contrast like a noir comic.',
		'Keep the exact composition, camera angle, framing, figure positions, figure sizes, poses and the light direction of image 1 – do not enlarge or move anything. Replace the plain mannequins with real characters and the simple blocks with real detailed places and things.',
		...who.map((w) => `In image 1, ${w}.`),
		// where we are – the render's grey hatching alone reads as stone tiles (sb-27-1/3 became a church inside)
		setting(sb, shot),
		useStyle && n < 3 ? `Image ${n + 1} is only a style reference for the drawing technique – do not copy its content.` : '',
		shot.prompt?.en ? `The moment: ${shot.prompt.en}` : '',
		`Light: one hard ${light.source && light.source !== 'scene' ? light.source : ''} light${light.key ? ` from the ${light.key}` : ''}${(light.soft ?? 0.5) < 0.3 ? ', no fill' : ', a little fill'}.`,
		'No text, no letters, no speech bubbles, no panel borders, no colour' + (light.accent?.color ? ` except ${light.accent.color} on ${light.accent.on ?? 'the accent'}` : '') + '.'
	].filter(Boolean).join(' ');
}

async function upload(file) {
	const fd = new FormData();
	fd.append('image', new Blob([readFileSync(file)]), `scutoons-${basename(file)}`);
	fd.append('overwrite', 'true');
	const r = await fetch(`${COMFY}/upload/image`, { method: 'POST', body: fd });
	if (!r.ok) throw new Error(`upload ${basename(file)}: ${r.status}`);
	return (await r.json()).name;
}

/** the API graph: Qwen-Image-Edit 2509 + Lightning, latent from image 1 (keeps its aspect) */
function graph(prompt, names, seed) {
	const g = {
		1: { class_type: 'UNETLoader', inputs: { unet_name: M.unet, weight_dtype: 'default' } },
		2: { class_type: 'CLIPLoader', inputs: { clip_name: M.clip, type: 'qwen_image', device: 'default' } },
		3: { class_type: 'VAELoader', inputs: { vae_name: M.vae } },
		4: { class_type: 'LoraLoaderModelOnly', inputs: { model: ['1', 0], lora_name: M.lora, strength_model: 1 } },
		5: { class_type: 'ModelSamplingAuraFlow', inputs: { model: M.lora === 'none' ? ['1', 0] : ['4', 0], shift: 3 } },
		6: { class_type: 'CFGNorm', inputs: { model: ['5', 0], strength: 1 } },
		20: { class_type: 'ImageScaleToTotalPixels', inputs: { image: ['10', 0], upscale_method: 'lanczos', megapixels: mp, resolution_steps: 16 } },
		21: { class_type: 'VAEEncode', inputs: { pixels: ['20', 0], vae: ['3', 0] } },
		30: { class_type: 'TextEncodeQwenImageEditPlus', inputs: { clip: ['2', 0], vae: ['3', 0], prompt, image1: ['20', 0] } },
		31: { class_type: 'TextEncodeQwenImageEditPlus', inputs: { clip: ['2', 0], vae: ['3', 0], prompt: '', image1: ['20', 0] } },
		40: { class_type: 'KSampler', inputs: { model: ['6', 0], positive: ['30', 0], negative: ['31', 0], latent_image: ['21', 0], seed, steps, cfg, sampler_name: 'euler', scheduler: 'simple', denoise: 1 } },
		41: { class_type: 'VAEDecode', inputs: { samples: ['40', 0], vae: ['3', 0] } },
		42: { class_type: 'SaveImage', inputs: { images: ['41', 0], filename_prefix: 'scutoons/storyboard' } }
	};
	names.forEach((n, i) => {
		g[10 + i] = { class_type: 'LoadImage', inputs: { image: n } };
		if (i > 0) { g[30].inputs[`image${i + 1}`] = [String(10 + i), 0]; g[31].inputs[`image${i + 1}`] = [String(10 + i), 0]; }
	});
	if (M.lora === 'none') delete g[4];
	if (refMethod) {
		g[32] = { class_type: 'FluxKontextMultiReferenceLatentMethod', inputs: { conditioning: ['30', 0], reference_latents_method: refMethod } };
		g[33] = { class_type: 'FluxKontextMultiReferenceLatentMethod', inputs: { conditioning: ['31', 0], reference_latents_method: refMethod } };
		g[40].inputs.positive = ['32', 0];
		g[40].inputs.negative = ['33', 0];
	}
	return g;
}

async function run(g) {
	const r = await fetch(`${COMFY}/prompt`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ prompt: g, client_id: 'scutoons-storyboard' }) });
	const j = await r.json();
	if (!r.ok) throw new Error(JSON.stringify(j.node_errors ?? j.error ?? j).slice(0, 500));
	for (;;) {
		await new Promise((ok) => setTimeout(ok, 2000));
		const h = (await (await fetch(`${COMFY}/history/${j.prompt_id}`)).json())[j.prompt_id];
		if (!h) continue;
		if (h.status?.status_str === 'error') throw new Error(JSON.stringify(h.status.messages?.find((m) => m[0] === 'execution_error')?.[1]?.exception_message ?? 'error'));
		const img = Object.values(h.outputs ?? {}).flatMap((o) => o.images ?? [])[0];
		if (img) return img;
	}
}

// --fresh: a headless ComfyUI from the desktop install for ONE image, then stopped (its memory never grows into the
// pagefile – see the header). Paths: the ComfyUI desktop app's defaults, or COMFY_PYTHON / COMFY_MAIN / COMFY_BASE /
// COMFY_FRONT / COMFY_MODELS.
const home = process.env.USERPROFILE ?? '', local = process.env.LOCALAPPDATA ?? '', roaming = process.env.APPDATA ?? '';
const app = resolve(local, 'Programs/ComfyUI/resources/ComfyUI');
const P = {
	python: process.env.COMFY_PYTHON ?? resolve(home, 'Documents/ComfyUI/.venv/Scripts/python.exe'),
	main: process.env.COMFY_MAIN ?? resolve(app, 'main.py'),
	base: process.env.COMFY_BASE ?? resolve(home, 'Documents/ComfyUI'),
	front: process.env.COMFY_FRONT ?? resolve(app, 'web_custom_versions/desktop_app'),
	models: process.env.COMFY_MODELS ?? resolve(roaming, 'ComfyUI/extra_models_config.yaml')
};
async function startServer() {
	const port = 8190;
	const log = openSync(resolve(OUT, 'comfy-server.log'), 'a');
	const child = spawn(P.python, [P.main, '--base-directory', P.base, '--user-directory', resolve(P.base, 'user'), '--input-directory', resolve(P.base, 'input'), '--output-directory', resolve(P.base, 'output'),
		'--extra-model-paths-config', P.models, '--front-end-root', P.front, '--listen', '127.0.0.1', '--port', String(port),
		'--disable-smart-memory', '--disable-pinned-memory', '--disable-mmap', '--cache-none', '--disable-auto-launch'], { stdio: ['ignore', log, log], windowsHide: true });
	COMFY = `http://127.0.0.1:${port}`;
	for (let i = 0; i < 120; i++) {
		if (await fetch(`${COMFY}/system_stats`).then((r) => r.ok).catch(() => false)) return child;
		await new Promise((ok) => setTimeout(ok, 2000));
	}
	stopServer(child);
	throw new Error(`the fresh ComfyUI did not start – ${resolve(OUT, 'comfy-server.log')}`);
}
function stopServer(child) {
	try { process.platform === 'win32' ? execFileSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], { stdio: 'ignore' }) : child.kill('SIGKILL'); } catch {}
}
const late = () => { if (!until) return false; const [h, m] = until.split(':').map(Number); const d = new Date(); return d.getHours() * 60 + d.getMinutes() >= h * 60 + m && d.getHours() < 20; };

/** Luma Agents API: the images inline (base64), the shot's aspect snapped to Luma's ratios; returns the PNG bytes */
const LUMA_RATIOS = ['3:1', '2:1', '16:9', '3:2', '1:1', '2:3', '9:16', '1:2', '1:3'];
const ratio = (s) => { const [a, b] = s.split(':').map(Number); return a / b; };
async function luma(prompt, files, aspect) {
	const key = process.env.LUMA_AGENTS_API_KEY;
	const h = { authorization: `Bearer ${key}`, 'content-type': 'application/json' };
	const want = ratio(aspect ?? '9:16');
	const aspect_ratio = LUMA_RATIOS.reduce((a, b) => (Math.abs(Math.log(ratio(b) / want)) < Math.abs(Math.log(ratio(a) / want)) ? b : a));
	const image_ref = files.map((f) => ({ data: readFileSync(f).toString('base64'), media_type: /\.webp$/i.test(f) ? 'image/webp' : /\.jpe?g$/i.test(f) ? 'image/jpeg' : 'image/png' }));
	const r = await fetch('https://agents.lumalabs.ai/v1/generations', { method: 'POST', headers: h, body: JSON.stringify({ prompt, model: lumaModel, type: 'image', aspect_ratio, style: lumaStyle, output_format: 'png', image_ref }) });
	let j = await r.json();
	if (!r.ok) throw new Error(`luma ${r.status}: ${JSON.stringify(j).slice(0, 300)}`);
	while (!['completed', 'failed'].includes(j.state)) {
		await new Promise((ok) => setTimeout(ok, 3000));
		j = await (await fetch(`https://agents.lumalabs.ai/v1/generations/${j.id}`, { headers: h })).json();
	}
	if (j.state === 'failed') throw new Error(`luma: ${j.failure_code ?? ''} ${j.failure_reason ?? ''}`);
	return { bytes: Buffer.from(await (await fetch(j.output.find((o) => o.type === 'image').url)).arrayBuffer()), id: j.id, aspect_ratio };
}

if (engine === 'luma' && !dry && !process.env.LUMA_AGENTS_API_KEY) { console.error('LUMA_AGENTS_API_KEY is not set – add it to .env (platform.lumalabs.ai → API keys).'); process.exit(2); }
if (engine === 'comfy' && !dry && !fresh) await fetch(`${COMFY}/system_stats`).catch(() => { console.error(`No ComfyUI at ${COMFY} – start the ComfyUI app.`); process.exit(2); });
mkdirSync(gen, { recursive: true });
let stopped = false;
const styles = styleFiles().map((f) => resolve(STYLE_DIR, f));
for (const { sb } of loadStoryboards(src)) for (const g of sb.groups ?? []) for (const shot of g.shots ?? []) {
	if (only ? !only.includes(shot.id) : !missing) continue;
	if (missing && existsSync(gen) && readdirSync(gen).some((f) => f.startsWith(`${shot.id}--`))) continue;
	if (late()) { if (!stopped) console.log(`${until} – no new shot`); stopped = true; continue; }
	if (shot.format === 'loop360') { console.log(`${shot.id}: a panorama – not yet`); continue; }
	const structure = resolve(OUT, 'clean', `${shot.id}.png`);
	if (!existsSync(structure)) { console.log(`${shot.id}: no render yet (render-shots.mjs)`); continue; }
	const chars = shotCharacters(sb, shot).map((c) => ({ ...c, place: placeOf(shot, c.fig) }));
	for (const c of chars) c.withImage = !!c.image && !c.place.small && chars.filter((d) => d.withImage).length < 2;
	const files = [structure, ...chars.filter((c) => c.withImage).map((c) => c.image.file)];
	if (useStyle && files.length < 3 && styles[0]) files.push(styles[0]);
	const prompt = instruction(sb, shot, chars);
	const seed = seedArg != null ? Number(seedArg) : Math.floor(Math.random() * 2 ** 31);
	if (dry) { console.log(`--- ${shot.id}: ${files.map((f) => basename(f)).join(', ')}\n${prompt}\n`); continue; }
	process.stdout.write(`${shot.id} … `);
	const t0 = Date.now();
	let server = null;
	try {
		let bytes, lumaJob;
		if (engine === 'luma') {
			lumaJob = await luma(prompt, files, shot.aspect);
			bytes = lumaJob.bytes;
		} else {
			if (fresh) server = await startServer();
			const names = [];
			for (const f of files) names.push(await upload(f));
			const img = await run(graph(prompt, names, seed));
			const r = await fetch(`${COMFY}/view?${new URLSearchParams({ filename: img.filename, subfolder: img.subfolder ?? '', type: img.type ?? 'output' })}`);
			bytes = Buffer.from(await r.arrayBuffer());
		}
		const n = readdirSync(gen).filter((f) => f.startsWith(`${shot.id}--`)).length + 1;
		const file = `${shot.id}--g${n}.png`;
		writeFileSync(resolve(gen, file), bytes);
		// black and white (the model tints despite the prompt) – unless the shot has its one colour accent
		const accent = { ...(sb.light ?? {}), ...(shot.light ?? {}) }.accent?.color;
		if (!accent) execFileSync('python', ['-I', '-c', 'import sys; from PIL import Image; Image.open(sys.argv[1]).convert("L").save(sys.argv[1])', resolve(gen, file)]);
		// let ComfyUI drop its models and caches: kept between jobs, its RAM grew until Windows paged the offloaded
		// weights and a step took 350 s instead of 90 (2026-10-10, 64 GB machine, 18 GB of Qwen offloaded)
		// only on the local 8 GB card – a cloud GPU keeps the model loaded for the whole session
		if (engine === 'comfy' && /127\.0\.0\.1|localhost/.test(COMFY)) await fetch(`${COMFY}/free`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ unload_models: true, free_memory: true }) }).catch(() => {});
		appendFileSync(resolve(OUT, 'generated.jsonl'), JSON.stringify({ date: new Date().toISOString(), shot: shot.id, file, ...(engine === 'luma' ? { engine: 'luma', model: lumaModel, style: lumaStyle, aspect: lumaJob.aspect_ratio, job: lumaJob.id } : { engine: 'comfy', model: M.unet, lora: M.lora, steps, cfg, refMethod, seed, host: new URL(COMFY).host }), images: files.map((f) => basename(f)), prompt }) + '\n');
		console.log(`${file} (${((Date.now() - t0) / 1000).toFixed(0)} s)`);
	} catch (e) {
		console.log(`failed: ${e.message}`);
	} finally {
		if (server) { stopServer(server); await new Promise((ok) => setTimeout(ok, 8000)); }
	}
}
