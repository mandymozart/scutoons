// The owner's verdict on a shot → the storyboard (`review`) and the dataset (owner 2026-10-09: refine the context
// from feedback; later train image models on real decisions). Approving records the shot as it is – the storyboard
// entry, its render, the chosen image (a Luma result, a photo, a drawing) and the prompt – in
// storyboards/dataset/manifest.jsonl (committed) with the files under storyboards/dataset/files/ (local, gitignored).
//
//   node scripts/storyboard/approve.mjs storyboards/catpope/s1c3 sb-28-3 [--image path/to/chosen.png] [--note "…"]
//   node scripts/storyboard/approve.mjs storyboards/catpope/s1c3 sb-28-3 --change "der Tiger hinter PLF, tiefere Kamera"

import { appendFileSync, copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, extname, resolve } from 'node:path';
import { ROOT } from './lib.mjs';
import { loadStoryboards } from './storyboard.mjs';

const args = process.argv.slice(2);
const take = (f) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : null; };
const image = take('--image'), note = take('--note'), change = take('--change');
const [src, id] = args;
if (!src || !id) { console.error('usage: approve.mjs <storyboard folder> <shot id> [--image file] [--note text] | --change text'); process.exit(2); }

const name = basename(resolve(src));
const hit = loadStoryboards(src).find(({ sb }) => (sb.groups ?? []).some((g) => (g.shots ?? []).some((s) => s.id === id)));
if (!hit) { console.error(`no shot ${id} in ${src}`); process.exit(1); }
const today = new Date().toISOString().slice(0, 10);

// 1. the review in the storyboard file (text replace keeps the file's own formatting)
const raw = readFileSync(hit.file, 'utf8');
const doc = JSON.parse(raw);
const shot = doc.groups.flatMap((g) => g.shots ?? []).find((s) => s.id === id);
shot.review = change ? { status: 'change', note: change, date: today } : { status: 'ok', ...(note ? { note } : {}), date: today };
writeFileSync(hit.file, JSON.stringify(doc, null, 2) + '\n');

// 2. the dataset (approvals only)
if (!change) {
	const dir = resolve(ROOT, 'storyboards/dataset/files', name, id);
	mkdirSync(dir, { recursive: true });
	const render = resolve(ROOT, 'storyboards/_out', name, 'frames', `${id}.png`);
	if (existsSync(render)) copyFileSync(render, resolve(dir, 'render.png'));
	if (image) copyFileSync(image, resolve(dir, `chosen${extname(image)}`));
	const prompt = resolve(ROOT, 'storyboards/_out', name, 'refpacks', id, 'prompt.txt');
	if (existsSync(prompt)) copyFileSync(prompt, resolve(dir, 'prompt.txt'));
	const cast = Object.fromEntries((doc.cast ?? []).map((c) => [c.ref, c]));
	appendFileSync(resolve(ROOT, 'storyboards/dataset/manifest.jsonl'), JSON.stringify({
		date: today, toon: doc.toon, chapter: doc.chapter, scene: doc.source?.scene, shot: id,
		light: { ...(doc.light ?? {}), ...(shot.light ?? {}) },
		cast: (shot.figures ?? []).map((f) => ({ ...cast[f.cast], persona: f.persona ?? cast[f.cast]?.persona, mood: f.mood, pose: f.pose })),
		storyboard: shot, files: { render: existsSync(render), chosen: image ? `chosen${extname(image)}` : null }, note
	}) + '\n');
}
console.log(`${id}: ${shot.review.status}${change ? ` – ${change}` : ''}${!change ? ' → dataset' : ''}`);
