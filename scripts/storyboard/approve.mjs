// The owner's verdict on a shot → the storyboard (`review`) and the dataset (owner 2026-10-09: refine the context
// from feedback; later train image models on real decisions). Approving records the shot as it is – the storyboard
// entry, its render, the chosen image (a Luma result, a photo, a drawing) and the prompt – in
// storyboards/dataset/manifest.jsonl (committed) with the files under storyboards/dataset/files/ (local, gitignored).
// The line names its chapter folder (`ch`) and, for a chosen file, which generated file it was:
// `files.from: "<shot>--g<n>.<ext>"` (null = not one of the shot's results) – the chosen per kind (chosen.png /
// chosen.mp4) the studio reads (clips.mjs approvedOf; agents/research/VARIANTS-CANVAS.md §3.5).
//
//   node scripts/storyboard/approve.mjs storyboards/catpope/s1c3 sb-28-3 [--image path/to/chosen.png] [--note "…"]
//   node scripts/storyboard/approve.mjs storyboards/catpope/s1c3 sb-28-3 --change "der Tiger hinter PLF, tiefere Kamera"

import { appendFileSync, copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, dirname, extname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ROOT } from './lib.mjs';
import { loadStoryboards } from './storyboard.mjs';

/** the generated file an image is – "<shot>--g<n>.<ext>" in storyboards/_out/<ch>/generated/ – or null */
function generatedName(image, ch, id) {
	const p = resolve(image);
	const name = basename(p);
	const inGenerated = basename(dirname(p)) === 'generated' && basename(dirname(dirname(p))) === ch && basename(dirname(dirname(dirname(p)))) === '_out';
	return inGenerated && name.startsWith(`${id}--`) && /--g\d+\.\w+$/.test(name) ? name : null;
}

/** approves (image / note) or asks for a change; root = the repo (the dataset's place); returns the console line */
export function approve({ root = ROOT, src, id, image = null, note = null, change = null, today = new Date().toISOString().slice(0, 10) }) {
	const name = basename(resolve(src));
	const hit = loadStoryboards(src).find(({ sb }) => (sb.groups ?? []).some((g) => (g.shots ?? []).some((s) => s.id === id)));
	if (!hit) throw new Error(`no shot ${id} in ${src}`);

	// 1. the review in the storyboard file (text replace keeps the file's own formatting)
	const raw = readFileSync(hit.file, 'utf8');
	const doc = JSON.parse(raw);
	const shot = doc.groups.flatMap((g) => g.shots ?? []).find((s) => s.id === id);
	shot.review = change ? { status: 'change', note: change, date: today } : { status: 'ok', ...(note ? { note } : {}), date: today };
	writeFileSync(hit.file, JSON.stringify(doc, null, 2) + '\n');

	// 2. the dataset (approvals only)
	if (!change) {
		const dir = resolve(root, 'storyboards/dataset/files', name, id);
		mkdirSync(dir, { recursive: true });
		const render = resolve(root, 'storyboards/_out', name, 'frames', `${id}.png`);
		if (existsSync(render)) copyFileSync(render, resolve(dir, 'render.png'));
		if (image) copyFileSync(image, resolve(dir, `chosen${extname(image)}`));
		const prompt = resolve(root, 'storyboards/_out', name, 'refpacks', id, 'prompt.txt');
		if (existsSync(prompt)) copyFileSync(prompt, resolve(dir, 'prompt.txt'));
		const cast = Object.fromEntries((doc.cast ?? []).map((c) => [c.ref, c]));
		appendFileSync(resolve(root, 'storyboards/dataset/manifest.jsonl'), JSON.stringify({
			date: today, toon: doc.toon, chapter: doc.chapter, ch: name, scene: doc.source?.scene, shot: id,
			light: { ...(doc.light ?? {}), ...(shot.light ?? {}) },
			cast: (shot.figures ?? []).map((f) => ({ ...cast[f.cast], persona: f.persona ?? cast[f.cast]?.persona, mood: f.mood, pose: f.pose })),
			storyboard: shot,
			files: { render: existsSync(render), chosen: image ? `chosen${extname(image)}` : null, ...(image ? { from: generatedName(image, name, id) } : {}) },
			note
		}) + '\n');
	}
	return `${id}: ${shot.review.status}${change ? ` – ${change}` : ''}${!change ? ' → dataset' : ''}`;
}

const cli = process.argv[1] && resolve(process.argv[1]).toLowerCase() === fileURLToPath(import.meta.url).toLowerCase();
if (cli) {
	const args = process.argv.slice(2);
	const take = (f) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : null; };
	const image = take('--image'), note = take('--note'), change = take('--change');
	const [src, id] = args;
	if (!src || !id) { console.error('usage: approve.mjs <storyboard folder> <shot id> [--image file] [--note text] | --change text'); process.exit(2); }
	try {
		console.log(approve({ src, id, image, note, change }));
	} catch (e) {
		console.error(e.message);
		process.exit(1);
	}
}
