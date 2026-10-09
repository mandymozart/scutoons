// Prompts for the gaps (STORYBOARD.md §9 M3, owner 2026-10-09: manual prompting in Luma this month): per shot that
// wants "generated" (or all shots with --all) a ready-to-paste prompt in English, the aspect to set, what to attach as
// references, and the safe-format check. Markdown sheet; nothing is sent anywhere.
//
//   node scripts/storyboard/prompts.mjs storyboards/catpope/s1c3 [--all] [--out storyboards/_out]

import { mkdirSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { ROOT, S, SAFE, W, parseAspect, textOf } from './lib.mjs';
import { loadStoryboards, shotsOf } from './storyboard.mjs';

const args = process.argv.slice(2);
const flag = (f) => { const i = args.indexOf(f); if (i < 0) return false; args.splice(i, 1); return true; };
const take = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : d; };
const all = flag('--all');
const outRoot = resolve(ROOT, take('--out', 'storyboards/_out'));
const src = args[0];
if (!src) { console.error('usage: prompts.mjs <storyboard folder> [--all]'); process.exit(2); }

/** The toon's style block (TREATMENTS.md open decision – a first version from chapters 1 + 2, to be confirmed). */
const STYLE = {
	catpope: 'Photographic, real location, natural light, high detail, cinematic still from a European art film; muted, slightly desaturated colours; no illustration, no comic or anime style, no text, no lettering, no watermark.'
};
const SIZE = { ecu: 'extreme close-up', cu: 'close-up', mcu: 'medium close-up', medium: 'medium shot', full: 'full shot', wide: 'wide shot', ews: 'extreme wide establishing shot' };
const ANGLE = { eye: 'eye level', high: 'high angle', low: 'low angle', bird: "bird's-eye view", worm: "worm's-eye view", dutch: 'dutch angle' };
const PROJ = { 'top-down': 'straight top-down view', isometric: 'isometric view', polar: 'little-planet polar projection (stereographic)', perspective: '' };
const PERSONA = {
	'catpope/human': 'a person in a long plain white papal robe (PLF)',
	'catpope/mountaintiger': 'a huge white tiger with enormous teeth',
	'catpope/cat': 'a hairless sphynx cat',
	'catpope/regular': 'a figure with a sphynx cat head in a papal robe (Cat Pope)'
};

function airPhrase(air) {
	return (air ?? []).map(([f, t]) => {
		const zone = f === 0 ? `the top ${t} %` : t === 100 ? `the bottom ${100 - f} %` : `a band from ${f} % to ${t} % down`;
		return `${zone} of the frame calm and empty (sky, snow, darkness or a plain field) – space for text`;
	}).join('; ');
}

const KEY = { left: 'from the left', right: 'from the right', top: 'from above', bottom: 'from below', front: 'from the front, behind the camera', back: 'from behind the subject (backlight)', 'top-left': 'from the upper left', 'top-right': 'from the upper right', 'bottom-left': 'from the lower left', 'bottom-right': 'from the lower right' };
const HEIGHT = { below: 'from below the face', low: 'low and grazing', eye: 'at eye level', high: 'high', overhead: 'straight overhead' };
function lightPhrase(l) {
	if (!l.key && !l.source) return 'Light: one extremely hard key light, deep black shadows, no fill.';
	const parts = [`Light: one extremely hard ${l.source && l.source !== 'none' ? `${l.source} ` : ''}key light ${KEY[l.key] ?? ''}${l.height ? `, ${HEIGHT[l.height]}` : ''}`.replace(/\s+/g, ' ').trim()];
	if (l.shadow === 'long') parts.push('long hard cast shadows');
	if (l.shadow === 'short') parts.push('short hard shadows');
	if (l.rim) parts.push('a bright rim of light around the figures against darkness');
	parts.push('crushed blacks, no fill light, no soft or diffuse light, high contrast');
	if (l.accent?.color) parts.push(`the only colour accent: ${l.accent.color}${l.accent.on ? ` (${l.accent.on})` : ''}`);
	return parts.join(', ') + '.';
}

const list = loadStoryboards(src);
const name = basename(resolve(src));
const md = [`# Prompts ${name} – ${all ? 'all shots' : 'the gaps (source.want = generated)'}`, '',
	'Manual prompting (Luma). Paste the prompt, set the aspect, attach the references. German = what the shot shows (storyboard); the prompt is English. Text and lettering are never in the image – they are live text boxes.', ''];
let n = 0;
for (const { sb } of list) {
	const cast = Object.fromEntries((sb.cast ?? []).map((c) => [c.ref, c]));
	const places = Object.fromEntries((sb.locations ?? []).map((l) => [l.ref, l]));
	for (const { shot } of shotsOf(sb)) {
		if (!all && shot.source?.want !== 'generated') continue;
		n++;
		const a = parseAspect(shot.aspect);
		const loc = places[shot.location];
		const who = (shot.figures ?? []).map((f) => {
			const c = cast[f.cast];
			const desc = c?.new ? c.new : PERSONA[`${c?.character}/${c?.persona}`] ?? c?.character ?? f.cast;
			const where = `${f.x < 0.4 ? 'left' : f.x > 0.6 ? 'right' : 'centre'}, ${f.y < 0.35 ? 'upper' : f.y > 0.65 ? 'lower' : 'middle'} part of the frame`;
			return `${desc}${f.mood ? `, ${f.mood}` : ''}${f.pose ? `, ${f.pose}` : ''}${f.facing ? `, facing ${f.facing}` : ''}, ${where}, about ${Math.round((f.h ?? 0.3) * 100)} % of the frame height`;
		});
		let format = '';
		let check = 'ok';
		if (shot.format === 'loop360') {
			const screens = (a.w / a.h) * (S / W);
			format = `Seamless 360° panorama, ${a.w}:${a.h}, the left and right edges must continue each other.`;
			if (screens > SAFE.loop360.maxScreens) check = `beyond ${SAFE.loop360.maxScreens} screens – split`;
			else if (screens > SAFE.loop360.plan) check = `above the planning width ${SAFE.loop360.plan} screens`;
		} else if (shot.format === 'vertical') {
			const tall = a.h / a.w;
			format = `Very tall vertical image ${shot.aspect}; composed to be scrolled from top to bottom (${shot.move ?? 'tilt-down'}).`;
			if (tall > SAFE.vertical.maxTall) check = `beyond 1:${SAFE.vertical.maxTall} – split`;
			else if (tall > 16 / 9 + 0.01) check = `taller than Luma's native 9:16 – generate 9:16 segments with overlap and outpaint, or ${tall > SAFE.vertical.plan ? 'split' : 'stitch'}`;
		} else format = 'Vertical phone frame 9:16.';
		const prompt = [
			format,
			shot.prompt?.en ?? '',
			((t) => (t ? t[0].toUpperCase() + t.slice(1) + '.' : ''))([SIZE[shot.shot?.size], ANGLE[shot.shot?.angle], PROJ[shot.projection]].filter(Boolean).join(', ')),
			loc ? `Location: ${loc.place ?? loc.new ?? loc.ref}${loc.time && loc.time !== 'interior' ? `, ${loc.time}` : ''}.` : '',
			who.length ? `Figures: ${who.join('; ')}.` : 'No people.',
			lightPhrase({ ...(sb.light ?? {}), ...(shot.light ?? {}) }),
			airPhrase(shot.air) ? `Composition: ${airPhrase(shot.air)}.` : '',
			STYLE[sb.toon] ?? ''
		].filter(Boolean).join(' ');
		const refs = [
			...(shot.figures ?? []).map((f) => cast[f.cast]).filter(Boolean).map((c) => c.new ? `${c.new} (no reference yet – Wanted)` : `${c.character} / ${c.persona}: the base views + a mood portrait`),
			loc ? `${loc.place ?? loc.ref}: the Figma selection on the chapter page (closest photo of the place)` : null,
			'the schematic as "layout only" (storyboards/_out/' + name + '/png/' + shot.id + '.png)'
		].filter(Boolean);
		md.push(`## ${shot.id} – ${shot.format} ${shot.aspect}`, '', `**Shows (storyboard):** ${textOf(shot.action)}`, '',
			'```text', prompt, '```', '', `**Aspect:** ${shot.format === 'loop360' ? `${a.w}:${a.h} panorama` : shot.aspect} · **safe format:** ${check}`, '',
			`**Attach:** ${[...new Set(refs)].join(' · ')}`, '', ...(shot.prompt?.en ? [] : ['**No `prompt.en` yet:** the German action holds the details – add them in English before prompting.', '']));
	}
}
mkdirSync(outRoot, { recursive: true });
const file = resolve(outRoot, `${name}-prompts.md`);
writeFileSync(file, md.join('\n'));
console.log(`${n} prompts → ${file}`);
