// The generated pass through Google's Gemini image model (owner 2026-10-10: "can we automate the luma step with an ai
// model … use the references and generate from the draft", "try gemini first"). Per shot it sends what a person
// attaches in Luma – the render without texts (the layout: composition, camera, poses, the hard light), the style
// pictures (storyboards/style/), each figure's character image (media dev copy, the shot's mood) – with the shot's
// prompt (luma.js), and drops the result where the studio's uploads go: _out/<ch>/generated/<id>--g<n>.png (the highest
// n + 1) with its v2 record in _out/<ch>/generated.jsonl (generated-log.mjs) – so the studio's variants board shows it,
// its ✓ (chosen) approves it, and figma-chapter.mjs places it.
//
// Costs money per image: it runs only on the shots named (--only) or at most --max (default 3) – never a whole chapter
// by accident. The key: GEMINI_API_KEY in the repo's .env (git-ignored; https://aistudio.google.com/apikey).
//
//   node scripts/storyboard/gemini.mjs storyboards/catpope/s1c3 --only sb-25-4,sb-28-3 [--model gemini-2.5-flash-image] [--dry]
//   node scripts/storyboard/gemini.mjs --models     the image models the key can use

import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { basename, extname, resolve } from 'node:path';
import { appendLine, keyOf, record, writeNew } from './generated-log.mjs';
import { ROOT, mediaRoot } from './lib.mjs';
import { lumaPrompt } from './luma.js';
import { STYLE_DIR, shotCharacters, styleFiles } from './refs.mjs';
import { loadStoryboards } from './storyboard.mjs';

const args = process.argv.slice(2);
const take = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : d; };
const flag = (f) => { const i = args.indexOf(f); return i >= 0 ? (args.splice(i, 1), true) : false; };
const only = take('--only', null)?.split(',');
const max = Number(take('--max', 3));
const model = take('--model', process.env.GEMINI_IMAGE_MODEL ?? 'gemini-2.5-flash-image');
const dry = flag('--dry');
const listModels = flag('--models');
const API = 'https://generativelanguage.googleapis.com/v1beta';

function key() {
	if (process.env.GEMINI_API_KEY) return process.env.GEMINI_API_KEY;
	const env = resolve(ROOT, '.env');
	const line = existsSync(env) && readFileSync(env, 'utf8').split(/\r?\n/).find((l) => /^\s*GEMINI_API_KEY\s*=/.test(l));
	const k = line && line.split('=').slice(1).join('=').trim().replace(/^["']|["']$/g, '');
	if (!k) { console.error('No GEMINI_API_KEY – put it into .env (https://aistudio.google.com/apikey).'); process.exit(2); }
	return k;
}
async function call(path, body) {
	const r = await fetch(`${API}/${path}`, { method: body ? 'POST' : 'GET', headers: { 'x-goog-api-key': key(), 'content-type': 'application/json' }, body: body ? JSON.stringify(body) : undefined });
	const j = await r.json().catch(() => ({}));
	if (!r.ok) throw new Error(`${r.status} ${j.error?.message ?? r.statusText}`);
	return j;
}

if (listModels) {
	const j = await call('models?pageSize=200');
	for (const m of j.models ?? []) if (/image/i.test(m.name)) console.log(m.name.replace('models/', ''), '–', m.displayName);
	process.exit(0);
}

const src = args[0];
if (!src) { console.error('usage: gemini.mjs <storyboard folder> --only <ids> [--max n] [--model m] [--dry] | --models'); process.exit(2); }
const ch = basename(resolve(src));
const OUT = resolve(ROOT, 'storyboards/_out', ch);
const MIME = { '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.webp': 'image/webp' };
const part = (file) => ({ inline_data: { mime_type: MIME[extname(file).toLowerCase()] ?? 'image/png', data: readFileSync(file).toString('base64') } });
// the ratios the model can make; a shot in another one (1:3 …) follows its structure image instead
const RATIOS = ['1:1', '2:3', '3:2', '3:4', '4:3', '4:5', '5:4', '9:16', '16:9', '21:9'];

const shots = [];
for (const { sb } of loadStoryboards(src)) for (const g of sb.groups ?? []) for (const shot of g.shots ?? []) {
	if (only ? only.includes(shot.id) : true) shots.push({ sb, shot });
}
const todo = shots.filter(({ shot }) => {
	if (shot.format === 'loop360') { console.log(`${shot.id}: a panorama – not yet (Gemini makes at most 21:9)`); return false; }
	return true;
});
if (todo.length > max) { console.log(`${todo.length} shots – only the first ${max} (--max ${todo.length} for all; each image costs)`); todo.length = max; }

const gen = resolve(OUT, 'generated');
mkdirSync(gen, { recursive: true });
const styles = styleFiles().map((f) => resolve(STYLE_DIR, f));
for (const { sb, shot } of todo) {
	const structure = resolve(OUT, 'clean', `${shot.id}.png`);
	if (!existsSync(structure)) { console.log(`${shot.id}: no render yet (render-shots.mjs)`); continue; }
	const chars = shotCharacters(sb, shot).filter((c) => c.image);
	// the images in order, each named in the text so the model knows its role
	const images = [
		{ file: structure, role: 'IMAGE 1 – THE LAYOUT (our 3D render): keep its exact composition, camera angle, figure positions and poses, and the light direction and shadow shapes. It is a rough mannequin render – do NOT copy its look, its crosshatching or its plain mannequin figures.' },
		...styles.map((f, i) => ({ file: f, role: `IMAGE ${2 + i} – THE STYLE: draw in this look.` })),
		...chars.map((c, i) => ({ file: c.image.file, role: `IMAGE ${2 + styles.length + i} – THE CHARACTER "${c.fig.cast}" (${c.cast.character ?? ''}${c.cast.persona ? ` / ${c.cast.persona}` : ''}, mood ${c.image.mood}): draw this figure exactly like this character – face, costume, proportions.` }))
	];
	const text = [
		'Make ONE finished storyboard frame for a vertical-scroll webtoon from the images below.',
		...images.map((i) => i.role),
		'No text, no letters, no speech bubbles, no frame borders, no captions in the image – the texts are added later.',
		'',
		lumaPrompt(sb, shot, {})
	].join('\n');
	const ratio = RATIOS.includes(shot.aspect) ? shot.aspect : null;
	if (dry) { console.log(`--- ${shot.id} (${shot.aspect}${ratio ? '' : ' → follows the layout image'}) · ${images.length} images: ${images.map((i) => basename(i.file)).join(', ')}\n${text.slice(0, 600)}…\n`); continue; }
	process.stdout.write(`${shot.id} … `);
	const t0 = Date.now();
	try {
		const j = await call(`models/${model}:generateContent`, {
			contents: [{ role: 'user', parts: [{ text }, ...images.map((i) => part(i.file))] }],
			generationConfig: { responseModalities: ['IMAGE'], ...(ratio ? { imageConfig: { aspectRatio: ratio } } : {}) }
		});
		const img = j.candidates?.[0]?.content?.parts?.find((p) => p.inlineData ?? p.inline_data);
		const data = img && (img.inlineData ?? img.inline_data);
		if (!data) { console.log(`no image (${j.candidates?.[0]?.finishReason ?? j.promptFeedback?.blockReason ?? 'empty answer'})`); continue; }
		const ext = /jpeg/.test(data.mimeType ?? data.mime_type ?? '') ? 'jpg' : 'png';
		const outs = resolve(ROOT, 'storyboards/_out');
		const { id, file } = writeNew(outs, ch, shot.id, ext, Buffer.from(data.data, 'base64'));
		// the record: image 1 the layout reference, then the style pictures, then the figures
		const roots = { out: outs, boards: resolve(ROOT, 'storyboards'), media: mediaRoot() };
		const inputs = images.map((i, n) => { const key = keyOf(roots, i.file); return { role: n === 0 ? 'reference' : styles.includes(i.file) ? 'style' : 'character', key, ...(key ? {} : { name: basename(i.file) }) }; });
		appendLine(outs, ch, record({ id, shot: shot.id, file, kind: 'image', generator: 'gemini', model, mode: 'image', status: 'succeeded', inputs, prompt: text, ratio, date: new Date().toISOString() }));
		console.log(`${file} (${((Date.now() - t0) / 1000).toFixed(0)} s)`);
	} catch (e) {
		console.log(`failed: ${e.message}`);
		if (/^(400|401|403|429)/.test(e.message)) break; // key / billing / quota: the rest would fail the same way
	}
}
