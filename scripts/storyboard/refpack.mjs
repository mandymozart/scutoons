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

import { copyFileSync, existsSync, mkdirSync, readdirSync, writeFileSync } from 'node:fs';
import { basename, extname, resolve } from 'node:path';
import { ROOT, mediaRoot, textOf } from './lib.mjs';
import { loadStoryboards } from './storyboard.mjs';

const args = process.argv.slice(2);
const take = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : d; };
const only = take('--only', null)?.split(',');
const outRoot = resolve(ROOT, take('--out', 'storyboards/_out'));
const src = args[0];
if (!src) { console.error('usage: refpack.mjs <storyboard folder> [--only ids]'); process.exit(2); }
const name = basename(resolve(src));
const frames = resolve(outRoot, name, 'frames');
const styleDir = resolve(ROOT, 'storyboards/style');
const styles = existsSync(styleDir) ? readdirSync(styleDir).filter((f) => /\.(png|jpe?g|webp)$/i.test(f)) : [];
const media = mediaRoot();

/** the registry's moods (characters / moods entity) – the character images exist for these */
const MOODS = ['acceptance', 'angry', 'bargaining', 'confused', 'crying', 'denial', 'disgusted', 'dizzy', 'embarrassed', 'furious', 'happy', 'laughing', 'neutral', 'sad', 'scared', 'smug', 'surprised', 'thinking'];
/** free storyboard moods → the nearest registry mood (until storyboards use the registry's words) */
const NEAR = {
	terrified: 'scared', panicked: 'scared', uneasy: 'confused', curious: 'confused', dazed: 'dizzy', clumsy: 'dizzy', falling: 'dizzy',
	exhausted: 'sad', cold: 'sad', nostalgic: 'sad', desperate: 'crying', despairing: 'crying',
	determined: 'smug', driven: 'smug', knowing: 'smug', focused: 'thinking', reverent: 'thinking',
	fierce: 'furious', hunting: 'furious', charging: 'furious', menacing: 'furious', impatient: 'angry', frustrated: 'angry', urgent: 'angry',
	awe: 'surprised', entranced: 'surprised', excited: 'happy', thrilled: 'happy', ecstatic: 'laughing', delighted: 'happy', eager: 'happy', playful: 'happy',
	relieved: 'acceptance', hopeful: 'acceptance', mocking: 'laughing'
};
const moodOf = (m) => (MOODS.includes(m) ? m : NEAR[m] ?? 'neutral');

function characterImage(c, framing, mood) {
	if (!media || !c?.character) return null;
	const folder = !c.persona || c.persona === 'regular' ? c.character : `${c.character}-${c.persona}`;
	const views = ['cu', 'ecu', 'mcu'].includes(framing) ? ['portrait', 'avatar', 'full'] : ['full', 'portrait'];
	for (const view of views) for (const m of [moodOf(mood), 'neutral']) for (const ext of ['webp', 'png']) {
		const f = resolve(media, 'characters', folder, `${folder}_${view}-${m}.${ext}`);
		if (existsSync(f)) return { file: f, mood: m };
	}
	return null;
}

const PERSONA = {
	'catpope/human': 'Cat Pope in human form (PLF): the long white papal robe with the cape, a sphynx cat head',
	'catpope/mountaintiger': 'the white mountain tiger with glowing blue rune markings and blue eyes (Cat Pope himself, as PLF fights himself)',
	'catpope/regular': 'Cat Pope (the narrator): sphynx cat head, papal robe',
	'catpope/cat': 'a hairless sphynx cat',
	'crows/regular': 'crow creatures, black, big beaks'
};
const VIEW = { belief: 'as the character perceives it', memory: 'a memory, more distant', narration: 'the narrator, outside the scene', real: '' };

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
		styles.forEach((f, i) => copyFileSync(resolve(styleDir, f), resolve(dir, `2-style-${i + 1}${extname(f)}`)));
		const who = [];
		for (const fig of shot.figures ?? []) {
			const c = { ...(cast[fig.cast] ?? {}), ...(fig.persona ? { persona: fig.persona } : {}) };
			const hit = characterImage(c, fig.framing, fig.mood);
			const img = hit?.file;
			const mood = moodOf(fig.mood);
			if (img) copyFileSync(img, resolve(dir, `3-${fig.cast}-${c.persona ?? 'regular'}-${hit.mood}${extname(img)}`));
			who.push(`${PERSONA[`${c.character}/${c.persona}`] ?? c.new ?? c.character ?? fig.cast} – ${mood}${fig.mood && fig.mood !== mood ? ` (${fig.mood})` : ''}${fig.pose ? `, ${fig.pose}` : ''}${img ? '' : ' [no character image yet – the render only]'}`);
		}
		const light = { ...(sb.light ?? {}), ...(shot.light ?? {}) };
		const prompt = [
			'Keep the exact composition, camera, poses and the hard light of the structure image (image reference).',
			`Render it in the look of the style reference: high-contrast black and white ink, deep solid black shadows, white highlights, no grey gradients, posterised${light.accent?.color ? `, the only colour: ${light.accent.color}${light.accent.on ? ` on ${light.accent.on}` : ''}` : ', no colour'}.`,
			who.length ? `Characters (character references): ${who.join('; ')}.` : 'No characters.',
			shot.prompt?.en ? `The scene: ${shot.prompt.en}` : `The scene (German storyboard – translate the key details): ${textOf(shot.action)}`,
			shot.view && VIEW[shot.view] ? `Perspective: ${VIEW[shot.view]}.` : '',
			light.note?.de ? `Light (storyboard): ${light.note.de}` : '',
			'No text, no lettering, no speech bubbles.'
		].filter(Boolean).join('\n');
		writeFileSync(resolve(dir, 'prompt.txt'), `${shot.id} – ${shot.format} ${shot.aspect}\n\n${prompt}\n\nLuma: image reference = 1-structure.png (strong), style reference = 2-style-*, character references = 3-*.\nAspect: ${shot.format === 'loop360' ? `${shot.aspect} panorama (seamless – generate wide or in parts)` : shot.aspect}\n`);
		packs++;
	}
}
console.log(`${packs} reference packs → ${resolve(outRoot, name, 'refpacks')}`);
console.log(`style: ${styles.length ? styles.join(', ') : 'none – put pictures into storyboards/style/'}`);
if (missing.length) console.log(`no render yet for ${missing.length} shot(s) – run tools/storyboard-render/render-shots.mjs first`);
