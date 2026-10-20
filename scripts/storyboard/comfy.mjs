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
// The result lands where the studio's uploads go: _out/<ch>/generated/<id>--g<n>.png (the highest n + 1 – the variants
// board, its ✓ chosen, figma-chapter.mjs), its v2 record in _out/<ch>/generated.jsonl (generated-log.mjs: the inputs as
// keys with their roles, the seed, the settings). --from <shot>--g<n>.png improves on a result (agents/research/
// VARIANTS-CANVAS.md §3.6): that generated image is image 1 instead of the render, and the record's parent.
//
//   node scripts/storyboard/comfy.mjs storyboards/catpope/s1c3 --only sb-25-4,sb-28-3 [--steps 4] [--seed n] [--mp 1] [--dry]
//   node scripts/storyboard/comfy.mjs storyboards/catpope/s1c3 --from sb-25-4--g3.png [--seed n]   (a file in
//        _out/<ch>/generated/, or its path; the shot is the file's)
//   node scripts/storyboard/comfy.mjs storyboards/catpope/s1c3 --missing --fresh [--until 08:30]   the night run: every
//        shot without an image yet, each on its own fresh headless ComfyUI (port 8190), no new shot after --until
//   COMFY_URL=http://127.0.0.1:8188 (default – the ComfyUI app, moved off the viewer's 8000)

import { execFileSync, spawn } from 'node:child_process';
import { openSync } from 'node:fs';
import { existsSync, mkdirSync, readdirSync, readFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { appendLine, keyOf, record, writeNew } from './generated-log.mjs';
import { ROOT, mediaRoot } from './lib.mjs';
import { STYLE_DIR, shotCharacters, styleFiles } from './refs.mjs';
import { loadStoryboards } from './storyboard.mjs';

// keys from the repo's .env (LUMA_AGENTS_API_KEY) – a variable already set in the shell wins
try { process.loadEnvFile(resolve(ROOT, '.env')); } catch {}
process.env.LUMA_AGENTS_API_KEY ??= process.env.LUMA_AGENT_API_KEY; // the owner's .env spells it singular

const args = process.argv.slice(2);
const take = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : d; };
const flag = (f) => { const i = args.indexOf(f); return i >= 0 ? (args.splice(i, 1), true) : false; };
let only = take('--only', null)?.split(',');
const fromArg = take('--from', null);
const steps = Number(take('--steps', 4));
// full quality on a big cloud GPU (2026-10-10, Runpod): COMFY_LORA=none --steps 40 --cfg 4 – no Lightning shortcut
const cfg = Number(take('--cfg', 1));
// how the reference images enter the model – 2511 wants 'index_timestep_zero' (ComfyUI's 2511 template); off = the node's default
const refMethod = take('--ref-method', null);
// a character LoRA on top (owner 2026-10-10: the PLF human LoRA from the 33 photos, trigger plf_human – Runpod / ai-toolkit):
// --char-lora plf_human_qwen_v1.safetensors[:0.8] (in ComfyUI's models/loras), the trigger goes in front of PLF's words
const [charLora, charStrength] = (take('--char-lora', '') || '').split(':');
const TRIGGERS = { 'catpope/human/default': 'plf_human' };
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
const OUTS = resolve(ROOT, 'storyboards/_out');
const OUT = resolve(OUTS, ch);
const gen = resolve(OUT, 'generated');
// the keys the record names its inputs by (generators/index.mjs): out:<ch>/… · media:… · boards:…
const roots = { out: OUTS, boards: resolve(ROOT, 'storyboards'), media: mediaRoot() };
// --from: a generated image of this chapter as image 1 – its shot is the only one
const from = fromArg ? resolve(gen, basename(fromArg)) : null;
if (from) {
	const shot = /^([\w-]+?)--g\d+\.(png|jpe?g|webp)$/i.exec(basename(from))?.[1];
	// a path names this chapter's file, never another chapter's with the same name
	const elsewhere = /[\\/]/.test(fromArg) &&resolve(fromArg).toLowerCase() !== from.toLowerCase();
	if (!shot || elsewhere || !existsSync(from)) { console.error(`--from takes a generated image of ${ch}: _out/${ch}/generated/<shot>--g<n>.png`); process.exit(2); }
	if (only && (only.length !== 1 || only[0] !== shot)) { console.error(`--from ${basename(from)} is shot ${shot} – --only names another`); process.exit(2); }
	only = [shot];
}

// PLF in the chapters is the human priest OR the sphynx cat – Cat Pope (the cat-headed pope) only where the story shows
// him, the mirror at the end of chapter 5 (owner 2026-10-10)
// keyed by the creator's ids (character / persona / look – the characters registry), owner 2026-10-10
const WHO = {
	'catpope/human/default': 'PLF, a human priest – a real human man with shoulder-length wavy blond hair with pink tips, clean-shaven, in a plain white cassock with a short white shoulder cape (mozzetta), no patterns on the robe – no cat features at all',
	'catpope/human/silicon': 'PLF, a human priest in a white papal cassock and short cape wearing a hairless sphynx-cat silicone mask with very large upright ears',
	'catpope/regular/default': 'Cat Pope: a priest in a white papal robe with the head of a hairless sphynx cat',
	'catpope/cat/default': 'a hairless sphynx cat – an animal on four legs',
	'catpope/mountaintiger/default': 'a HUGE white mountain tiger with a glowing rune sigil on the forehead and body'
};
const idOf = (c) => `${c.character}/${c.persona ?? 'regular'}/${c.look ?? 'default'}`;
const whoOf = (c) => `${charLora && TRIGGERS[idOf(c)] ? `${TRIGGERS[idOf(c)]}, ` : ''}${WHO[idOf(c)] ?? WHO[`${c.character}/${c.persona ?? 'regular'}/default`] ?? c.character}`;
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
		const name = whoOf(c.cast);
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

/** the brief the owner's Luma agent writes for a frame (storyboards/style/luma-skills/storyboard-frame-briefs.md, owner
 *  2026-10-10): fixed sections, a named role per image, identity in words, placement in numbers, light first, the fixed
 *  look and negatives – built from the storyboard's own data. --prompt short = the old paragraph above. */
const SIZE = { ecu: 'extreme close-up', cu: 'close-up', mcu: 'medium close-up', ms: 'medium shot', medium: 'medium shot', mws: 'medium wide shot', ws: 'wide shot', ews: 'extreme wide shot' };
const ANGLE = { low: 'low angle looking up', high: 'high angle looking down', eye: 'eye level', top: 'straight down from above', worm: "worm's-eye view" };
const FACING = { front: 'facing camera', away: 'BACK TO CAMERA', left: 'in PROFILE facing LEFT', right: 'in PROFILE facing RIGHT' };
// the look (owner 2026-10-10: 'lets give up on the line art … the photo noir is better'): photo noir by default, --look ink
const LOOKS = {
	photo: { kind: 'black-and-white PHOTO NOIR still', look: 'a black-and-white cinematic photograph, film noir — real people, real textures, deep solid blacks, crisp highlights, subtle film grain; a still from a live-action film, NOT a drawing, NOT line art' },
	ink: { kind: 'clean black-and-white LINE ART', look: 'clean crisp contour lines + solid black shadow shapes, intense noir black-and-white — NO hatching, grey, sketch, or gradients' }
};
const lookStyle = take('--look', 'photo');
const LOOK = LOOKS[lookStyle]?.look ?? LOOKS.photo.look;
const KIND = LOOKS[lookStyle]?.kind ?? LOOKS.photo.kind;
function brief(sb, shot, chars) {
	const light = { ...(sb.light ?? {}), ...(shot.light ?? {}) };
	const aspect = shot.aspect ?? '9:16';
	const where = (p) => p.where.toUpperCase().replace(/^(TOP|MIDDLE|BOTTOM)$/, 'CENTRE-$1').replace(/^(\w+) (\w+)$/, '$2-$1');
	let n = 1;
	const figs = chars.map((c) => {
		const name = whoOf(c.cast);
		const img = c.withImage ? ++n : null;
		const mood = c.fig.mood ? `${c.fig.mood}, ` : '';
		const pose = c.fig.pose ? `${c.fig.pose.replace(/-/g, ' ')}, ` : '';
		const facing = FACING[c.fig.facing] ? `${FACING[c.fig.facing]}, ` : '';
		return { name, img, text: `${name}${img ? ` (identity: image ${img})` : ''} — ${mood}${pose}${facing}${c.place.small ? 'SMALL, ' : ''}in the ${where(c.place)} at ~${c.place.pct}% of the height` };
	});
	const src = light.source && light.source !== 'scene' ? `${light.source} ` : '';
	const dir = light.height === 'below' || light.key === 'bottom' ? 'from BELOW (menacing)' : light.key ? `from the ${light.key.toUpperCase()}` : '';
	const fill = (light.soft ?? 0.5) < 0.3 ? 'NO fill — surfaces facing the light pure white, everything else solid black' : `PLUS a little fill (softness ~${light.soft ?? 0.5}) — shadow side dark but READABLE, solid black only in the deepest shadows`;
	const lightLine = `ONE extremely hard ${src}key light ${dir}${light.shadow === 'long' ? ', long hard cast shadows' : ''}, ${fill}; one clear direction, never flat`;
	const camera = `${SIZE[shot.shot?.size] ?? 'medium shot'}, ${ANGLE[shot.shot?.angle] ?? 'eye level'}, ${shot.projection === 'perspective' || !shot.projection ? 'normal perspective' : shot.projection}`;
	const accent = light.accent?.color ? ` except ${light.accent.color} on ${light.accent.on ?? 'the accent'}` : '';
	const roles = [`image 1 is the STRONG LAYOUT reference (our 3D render) — keep its EXACT composition, camera, poses, figure sizes and light placement; it IS the layout. Its plain mannequins and grey blocks are placeholders: replace them with the real characters and real, detailed places and things.`,
		...figs.filter((f) => f.img).map((f) => `image ${f.img} = ${f.name} — identity only (face, hair, costume); show them in this frame's look, never copy the reference's framing, background or light.`),
		useStyle && n < 3 ? `image ${n + 1} = a style reference for the drawing technique only — never copy its content.` : ''].filter(Boolean);
	return [
		`THIS JOB'S DELIVERABLE: One ${aspect} vertical-scroll webtoon storyboard frame, ${KIND}, matching the layout of image 1. ${figs.map((f) => f.text).join('. ')}. CAMERA: ${camera} (per the layout). LIGHT (most important): ${lightLine}. LOOK: ${LOOK}. No text, lettering, numbers, speech bubbles, panel borders, watermark, or colour${accent}.`,
		`EXPECTED OUTPUT: 1 image — ${aspect}. ${LOOK.split(' — ')[0]}. Keep composition/camera/light of image 1. ${figs.map((f) => f.text.split(' — ')[1]).join('; ')}.`,
		`WORKFLOW STEPS: 1. Follow image 1 exactly for composition, camera, poses and light. ${figs.map((f, i) => `${i + 2}. ${f.text}.`).join(' ')} ${figs.length + 2}. ${lightLine}. ${figs.length + 3}. ${LOOK}; no text, borders, watermark or colour.`,
		`REFERENCES: ${roles.join(' ')}`,
		`CONTEXT: shot ${shot.id}, ${aspect}. ${setting(sb, shot)}${shot.view === 'belief' ? ' Subjective view: the frame shows the threat as PLF believes it.' : ''}${shot.prompt?.en ? ` The moment: ${shot.prompt.en}` : ''}`
	].join('\n');
}
// Luma's content filter refuses some horror words (sb-28-2: 422 content_moderated twice) – softer synonyms for Luma only
const soften = (t) => t.replace(/jaws (wide )?open/gi, 'mouth open in a mighty roar').replace(/enormous teeth|huge teeth/gi, 'big white fangs').replace(/terrified/gi, 'startled').replace(/menacing/gi, 'dramatic').replace(/grotesque/gi, 'dramatic').replace(/nightmare/gi, 'vision');
const promptStyle = take('--prompt', 'brief');

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
	if (charLora) {
		g[7] = { class_type: 'LoraLoaderModelOnly', inputs: { model: g[5].inputs.model, lora_name: charLora, strength_model: Number(charStrength ?? 0.8) } };
		g[5].inputs.model = ['7', 0];
	}
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
	const structure = from ?? resolve(OUT, 'clean', `${shot.id}.png`);
	if (!existsSync(structure)) { console.log(`${shot.id}: no render yet (render-shots.mjs)`); continue; }
	const chars = shotCharacters(sb, shot).map((c) => ({ ...c, place: placeOf(shot, c.fig) }));
	for (const c of chars) c.withImage = !!c.image && !c.place.small && chars.filter((d) => d.withImage).length < 2;
	const files = [structure, ...chars.filter((c) => c.withImage).map((c) => c.image.file)];
	if (useStyle && files.length < 3 && styles[0]) files.push(styles[0]);
	const prompt = promptStyle === 'short' ? instruction(sb, shot, chars) : brief(sb, shot, chars);
	const seed = seedArg != null ? Number(seedArg) : Math.floor(Math.random() * 2 ** 31);
	if (dry) { console.log(`--- ${shot.id}: ${files.map((f) => basename(f)).join(', ')}\n${prompt}\n`); continue; }
	process.stdout.write(`${shot.id} … `);
	const t0 = Date.now();
	let server = null;
	try {
		let bytes, lumaJob;
		if (engine === 'luma') {
			lumaJob = await luma(soften(prompt), files, shot.aspect);
			bytes = lumaJob.bytes;
		} else {
			if (fresh) server = await startServer();
			const names = [];
			for (const f of files) names.push(await upload(f));
			const img = await run(graph(prompt, names, seed));
			const r = await fetch(`${COMFY}/view?${new URLSearchParams({ filename: img.filename, subfolder: img.subfolder ?? '', type: img.type ?? 'output' })}`);
			bytes = Buffer.from(await r.arrayBuffer());
		}
		const { id, file } = writeNew(OUTS, ch, shot.id, 'png', bytes);
		// the record, at once (a file never stays without one): image 1 is the picture edited (comfy) or the layout
		// reference (luma), then the figures, the style
		const role = (f, i) => (i === 0 ? (engine === 'luma' ? 'reference' : 'still') : styles.includes(f) ? 'style' : 'character');
		const inputs = files.map((f, i) => { const key = keyOf(roots, f); return { role: role(f, i), key, ...(key ? {} : { name: basename(f) }) }; });
		appendLine(OUTS, ch, record({
			id, shot: shot.id, file, kind: 'image', inputs, prompt, status: 'succeeded', date: new Date().toISOString(),
			...(engine === 'luma'
				? { generator: 'luma', model: lumaModel, mode: 'image', job: lumaJob.id, ratio: lumaJob.aspect_ratio, params: { style: lumaStyle } }
				: { generator: 'comfy-local', model: 'qwen-image-edit', mode: 'comfy', seed, params: { unet: M.unet, lora: M.lora, steps, cfg, refMethod, charLora: charLora || undefined, promptStyle, look: lookStyle, host: new URL(COMFY).host } })
		}));
		// black and white (the model tints despite the prompt) – unless the shot has its one colour accent
		const accent = { ...(sb.light ?? {}), ...(shot.light ?? {}) }.accent?.color;
		if (!accent) execFileSync('python', ['-I', '-c', 'import sys; from PIL import Image; Image.open(sys.argv[1]).convert("L").save(sys.argv[1])', resolve(gen, file)]);
		// let ComfyUI drop its models and caches: kept between jobs, its RAM grew until Windows paged the offloaded
		// weights and a step took 350 s instead of 90 (2026-10-10, 64 GB machine, 18 GB of Qwen offloaded)
		// only on the local 8 GB card – a cloud GPU keeps the model loaded for the whole session
		if (engine === 'comfy' && /127\.0\.0\.1|localhost/.test(COMFY)) await fetch(`${COMFY}/free`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ unload_models: true, free_memory: true }) }).catch(() => {});
		console.log(`${file} (${((Date.now() - t0) / 1000).toFixed(0)} s)`);
	} catch (e) {
		console.log(`failed: ${e.message}`);
	} finally {
		if (server) { stopServer(server); await new Promise((ok) => setTimeout(ok, 8000)); }
	}
}
