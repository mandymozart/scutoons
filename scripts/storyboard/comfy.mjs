// The generated pass on the owner's own GPU (owner 2026-10-10: "is there not free generator available?", "can we run
// something on my rtx2070 locally", "comfy ui is installed"): the local ComfyUI (desktop app, port 8000) with
// Qwen-Image-Edit 2509 + the Lightning 4-step LoRA – up to three input images: 1 = our render without texts (the
// layout: composition, camera, poses, the hard light), 2–3 = the figures' character images (or a style picture when
// there is no figure). Free; the rehearsal of the cloud-GPU chain (agents/STORYBOARD.md §9).
//
// The result lands where the studio's uploads go: _out/<ch>/generated/<id>--g<n>.png (Variants, Approve as chosen,
// figma-chapter.mjs), logged in _out/<ch>/generated.jsonl.
//
//   node scripts/storyboard/comfy.mjs storyboards/catpope/s1c3 --only sb-25-4,sb-28-3 [--steps 4] [--seed n] [--mp 1] [--dry]
//   COMFY_URL=http://127.0.0.1:8188 (default – the ComfyUI app, moved off the viewer's 8000)

import { execFileSync } from 'node:child_process';
import { appendFileSync, existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { ROOT } from './lib.mjs';
import { STYLE_DIR, shotCharacters, styleFiles } from './refs.mjs';
import { loadStoryboards } from './storyboard.mjs';

const args = process.argv.slice(2);
const take = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : d; };
const flag = (f) => { const i = args.indexOf(f); return i >= 0 ? (args.splice(i, 1), true) : false; };
const only = take('--only', null)?.split(',');
const steps = Number(take('--steps', 4));
const mp = Number(take('--mp', 1));
const seedArg = take('--seed', null);
const dry = flag('--dry');
const COMFY = process.env.COMFY_URL ?? 'http://127.0.0.1:8188';
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

const WHO = { 'catpope/human': 'Cat Pope as a human woman (PLF) in a long white papal robe', 'catpope/regular': 'Cat Pope, a hairless sphynx cat pope in a white papal robe', 'catpope/mountaintiger': 'a huge white tiger with blue runes' };
/** Qwen-Edit wants a short, direct instruction – not the long Luma brief */
function instruction(sb, shot, chars) {
	const light = { ...(sb.light ?? {}), ...(shot.light ?? {}) };
	const who = chars.map((c, i) => `the ${c.fig.pose ?? 'standing'} figure${chars.length > 1 ? ` ${i + 1}` : ''} is ${WHO[`${c.cast.character}/${c.cast.persona ?? 'regular'}`] ?? c.cast.character} – look exactly like the character in image ${i + 2}${c.image?.mood ? `, ${c.image.mood} expression` : ''}`);
	return [
		'Redraw image 1 as a finished black-and-white webtoon storyboard frame: clean, crisp ink line art on white paper, solid black shadows, high contrast like a noir comic.',
		'Keep the exact composition, camera angle, framing, figure positions, poses and the light direction of image 1. Replace the plain mannequins with real characters and the simple blocks with real detailed places and things.',
		...who.map((w) => `In image 1, ${w}.`),
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
		5: { class_type: 'ModelSamplingAuraFlow', inputs: { model: ['4', 0], shift: 3 } },
		6: { class_type: 'CFGNorm', inputs: { model: ['5', 0], strength: 1 } },
		20: { class_type: 'ImageScaleToTotalPixels', inputs: { image: ['10', 0], upscale_method: 'lanczos', megapixels: mp, resolution_steps: 16 } },
		21: { class_type: 'VAEEncode', inputs: { pixels: ['20', 0], vae: ['3', 0] } },
		30: { class_type: 'TextEncodeQwenImageEditPlus', inputs: { clip: ['2', 0], vae: ['3', 0], prompt, image1: ['20', 0] } },
		31: { class_type: 'TextEncodeQwenImageEditPlus', inputs: { clip: ['2', 0], vae: ['3', 0], prompt: '', image1: ['20', 0] } },
		40: { class_type: 'KSampler', inputs: { model: ['6', 0], positive: ['30', 0], negative: ['31', 0], latent_image: ['21', 0], seed, steps, cfg: 1, sampler_name: 'euler', scheduler: 'simple', denoise: 1 } },
		41: { class_type: 'VAEDecode', inputs: { samples: ['40', 0], vae: ['3', 0] } },
		42: { class_type: 'SaveImage', inputs: { images: ['41', 0], filename_prefix: 'scutoons/storyboard' } }
	};
	names.forEach((n, i) => {
		g[10 + i] = { class_type: 'LoadImage', inputs: { image: n } };
		if (i > 0) { g[30].inputs[`image${i + 1}`] = [String(10 + i), 0]; g[31].inputs[`image${i + 1}`] = [String(10 + i), 0]; }
	});
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

if (!dry) await fetch(`${COMFY}/system_stats`).catch(() => { console.error(`No ComfyUI at ${COMFY} – start the ComfyUI app.`); process.exit(2); });
mkdirSync(gen, { recursive: true });
const styles = styleFiles().map((f) => resolve(STYLE_DIR, f));
for (const { sb } of loadStoryboards(src)) for (const g of sb.groups ?? []) for (const shot of g.shots ?? []) {
	if (!only?.includes(shot.id)) continue;
	if (shot.format === 'loop360') { console.log(`${shot.id}: a panorama – not yet`); continue; }
	const structure = resolve(OUT, 'clean', `${shot.id}.png`);
	if (!existsSync(structure)) { console.log(`${shot.id}: no render yet (render-shots.mjs)`); continue; }
	const chars = shotCharacters(sb, shot).filter((c) => c.image).slice(0, 2);
	const files = [structure, ...chars.map((c) => c.image.file)];
	if (files.length < 3 && styles[0]) files.push(styles[0]);
	const prompt = instruction(sb, shot, chars);
	const seed = seedArg != null ? Number(seedArg) : Math.floor(Math.random() * 2 ** 31);
	if (dry) { console.log(`--- ${shot.id}: ${files.map((f) => basename(f)).join(', ')}\n${prompt}\n`); continue; }
	process.stdout.write(`${shot.id} … `);
	const t0 = Date.now();
	try {
		const names = [];
		for (const f of files) names.push(await upload(f));
		const img = await run(graph(prompt, names, seed));
		const r = await fetch(`${COMFY}/view?${new URLSearchParams({ filename: img.filename, subfolder: img.subfolder ?? '', type: img.type ?? 'output' })}`);
		const n = readdirSync(gen).filter((f) => f.startsWith(`${shot.id}--`)).length + 1;
		const file = `${shot.id}--g${n}.png`;
		writeFileSync(resolve(gen, file), Buffer.from(await r.arrayBuffer()));
		// black and white (the model tints despite the prompt) – unless the shot has its one colour accent
		const accent = { ...(sb.light ?? {}), ...(shot.light ?? {}) }.accent?.color;
		if (!accent) execFileSync('python', ['-I', '-c', 'import sys; from PIL import Image; Image.open(sys.argv[1]).convert("L").save(sys.argv[1])', resolve(gen, file)]);
		appendFileSync(resolve(OUT, 'generated.jsonl'), JSON.stringify({ date: new Date().toISOString(), shot: shot.id, file, engine: 'comfy', model: M.unet, lora: M.lora, steps, seed, images: files.map((f) => basename(f)), prompt }) + '\n');
		console.log(`${file} (${((Date.now() - t0) / 1000).toFixed(0)} s)`);
	} catch (e) {
		console.log(`failed: ${e.message}`);
	}
}
