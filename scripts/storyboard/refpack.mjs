// A reference pack per shot (owner 2026-10-09: "run these basic 3d scenes through an image generator with a style
// reference … and feed the characters from the moods"): one folder per shot with
//   1-structure.png        the render (tools/storyboard-render) – composition, poses, camera, the hard light
//   2-style-*.{png,webp}   every picture in storyboards/style/ (the look)
//   3-<who>-<persona>-<mood>.webp   each figure's character image (full body or portrait by framing, the shot's mood
//                          mapped to the registry's moods) – from the media dev copy; production has the truth
//   prompt.txt             what to paste (structure / style / character roles spelled out)
// For manual prompting in Luma today (image reference = 1, style reference = 2, character references = 3);
// the same pack feeds the cloud-GPU chain later (depth / lines from 1, style adapter from 2, character LoRA / refs 3).
//
//   node scripts/storyboard/refpack.mjs storyboards/catpope/s1c3 [--only sb-28-2,sb-28-3] [--out storyboards/_out]

import { copyFileSync, existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { basename, extname, resolve } from 'node:path';
import { ROOT, readEntity } from './lib.mjs';
import { lumaPrompt } from './luma.js';
import { STYLE_DIR, shotCharacters, styleFiles } from './refs.mjs';
import { loadStoryboards } from './storyboard.mjs';

const args = process.argv.slice(2);
const take = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : d; };
const only = take('--only', null)?.split(',');
const outRoot = resolve(ROOT, take('--out', 'storyboards/_out'));
const src = args[0];
if (!src) { console.error('usage: refpack.mjs <storyboard folder> [--only ids]'); process.exit(2); }
const name = basename(resolve(src));
const frames = resolve(outRoot, name, 'frames');
const styles = styleFiles();

// the script per scene (local dev DB, read-only) for the prompt's context; without the DB the prompt goes without it
let scriptText = '';
try { scriptText = readEntity(`script:${loadStoryboards(src)[0]?.sb.toon ?? 'catpope'}:de`).text ?? ''; } catch {}
function sceneScript(n) {
	const lines = scriptText.split('\n');
	const start = lines.findIndex((l) => new RegExp(`^${n}\\s{1,4}\\S`).test(l));
	if (start < 0) return '';
	let end = lines.findIndex((l, i) => i > start && (/^\d+\s{1,4}\S/.test(l) || /^\s+# \d+\. Kapitel/.test(l)));
	if (end < 0) end = lines.length;
	return lines.slice(start, end).join('\n');
}

let packs = 0, missing = [];
for (const { sb } of loadStoryboards(src)) {
	const cast = Object.fromEntries((sb.cast ?? []).map((c) => [c.ref, c]));
	for (const g of sb.groups ?? []) for (const shot of g.shots ?? []) {
		if (only && !only.includes(shot.id)) continue;
		const dir = resolve(outRoot, name, 'refpacks', shot.id);
		mkdirSync(dir, { recursive: true });
		const frame = resolve(frames, `${shot.id}.png`);
		if (existsSync(frame)) copyFileSync(frame, resolve(dir, '1-structure.png'));
		else missing.push(shot.id);
		styles.forEach((f, i) => copyFileSync(resolve(STYLE_DIR, f), resolve(dir, `2-style-${i + 1}${extname(f)}`)));
		const refs = { structure: '1-structure.png', style: styles.map((f, i) => `2-style-${i + 1}${extname(f)}`), characters: [] };
		for (const { fig, cast, image } of shotCharacters(sb, shot)) {
			if (!image) { refs.characters.push(`(none yet for ${fig.cast} / ${cast.persona ?? 'regular'} – the render only)`); continue; }
			const f = `3-${fig.cast}-${cast.persona ?? 'regular'}-${image.mood}${extname(image.file)}`;
			copyFileSync(image.file, resolve(dir, f));
			refs.characters.push(f);
		}
		writeFileSync(resolve(dir, 'prompt.txt'), lumaPrompt(sb, shot, { references: refs, script: sceneScript(sb.source?.scene) }));
		packs++;
	}
}
console.log(`${packs} reference packs → ${resolve(outRoot, name, 'refpacks')}`);
console.log(`style: ${styles.length ? styles.join(', ') : 'none – put pictures into storyboards/style/'}`);
if (missing.length) console.log(`no render yet for ${missing.length} shot(s) – run tools/storyboard-render/render-shots.mjs first`);
