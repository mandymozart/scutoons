// Prompts for the gaps (from scripts/storyboard/prompts.mjs, #119; STORYBOARD.md §9 M3): per shot that wants
// "generated" (or every shot) a ready-to-paste prompt in English, the aspect to set, what to attach as references, and
// the safe-format check. promptSheet() is the Markdown sheet the old script wrote; nothing is sent anywhere.

import { S, W, parseAspect } from './grid.ts';
import { shotsOf } from './measure.ts';
import type { Light, Shot, StoryboardDoc } from './types.ts';
import { SAFE, textOf } from './vocab.ts';

/** The toon's style block (TREATMENTS.md open decision – a first version from chapters 1 + 2, to be confirmed). */
export const STYLE: Record<string, string> = {
	catpope: 'Storyboard frame: clean line art, crisp contour lines on white, solid black shadow shapes from one extremely hard light so the lighting reads clearly; black and white only (one accent colour if named); no hatching, no grey shading, no sketchy or hand-drawn strokes, no text, no lettering, no watermark.',
	// the final images (photo collage look of chapters 1 + 2) get their own style block when the storyboard is approved
	catpopeFinal: 'Photographic, real location, cinematic still from a European art film; muted, slightly desaturated colours; no illustration, no comic or anime style, no text, no lettering, no watermark.'
};
const SIZE: Record<string, string> = { ecu: 'extreme close-up', cu: 'close-up', mcu: 'medium close-up', medium: 'medium shot', full: 'full shot', wide: 'wide shot', ews: 'extreme wide establishing shot' };
const ANGLE: Record<string, string> = { eye: 'eye level', high: 'high angle', low: 'low angle', bird: "bird's-eye view", worm: "worm's-eye view", dutch: 'dutch angle' };
const PROJ: Record<string, string> = { 'top-down': 'straight top-down view', isometric: 'isometric view', polar: 'little-planet polar projection (stereographic)', perspective: '' };
const PERSONA: Record<string, string> = {
	'catpope/human': 'a person in a long plain white papal robe (PLF)',
	'catpope/mountaintiger': 'a huge white tiger with enormous teeth',
	'catpope/cat': 'a hairless sphynx cat',
	'catpope/regular': 'a figure with a sphynx cat head in a papal robe (Cat Pope)'
};
const KEY: Record<string, string> = { left: 'from the left', right: 'from the right', top: 'from above', bottom: 'from below', front: 'from the front, behind the camera', back: 'from behind the subject (backlight)', 'top-left': 'from the upper left', 'top-right': 'from the upper right', 'bottom-left': 'from the lower left', 'bottom-right': 'from the lower right' };
const HEIGHT: Record<string, string> = { below: 'from below the face', low: 'low and grazing', eye: 'at eye level', high: 'high', overhead: 'straight overhead' };
const VIEW_LINE: Record<string, string> = { belief: 'Shown as the character perceives it (subjective, slightly unreal).', memory: 'A memory: the past, more distant and grainier.', narration: 'Narration: the narrator addresses the reader, outside the scene.' };
const get = (map: Record<string, string>, k: string | undefined) => (k === undefined ? undefined : map[k]);

/** the air zones as a composition phrase ('' for none) */
export function airPhrase(air: [number, number][] | undefined): string {
	return (air ?? []).map(([f, t]) => {
		const zone = f === 0 ? `the top ${t} %` : t === 100 ? `the bottom ${100 - f} %` : `a band from ${f} % to ${t} % down`;
		return `${zone} of the frame calm and empty (sky, snow, darkness or a plain field) – space for text`;
	}).join('; ');
}

/** the merged light as a prompt sentence */
export function lightPhrase(l: Light): string {
	if (!l.key && !l.source) return 'Light: one extremely hard key light, deep black shadows, no fill.';
	const parts = [`Light: one extremely hard ${l.source && l.source !== 'none' ? `${l.source} ` : ''}key light ${get(KEY, l.key) ?? ''}${l.height ? `, ${HEIGHT[l.height]}` : ''}`.replace(/\s+/g, ' ').trim()];
	if (l.shadow === 'long') parts.push('long hard cast shadows');
	if (l.shadow === 'short') parts.push('short hard shadows');
	if (l.rim) parts.push('a bright rim of light around the figures against darkness');
	parts.push('crushed blacks, no fill light, no soft or diffuse light, high contrast');
	if (l.accent?.color) parts.push(`the only colour accent: ${l.accent.color}${l.accent.on ? ` (${l.accent.on})` : ''}`);
	return parts.join(', ') + '.';
}

export interface GapPrompt {
	prompt: string;
	/** "ok" or what the safe-format envelope says */
	check: string;
	/** the aspect to set in the generator */
	aspect: string;
	/** what to attach, each once */
	refs: string[];
}

/** one shot's prompt; `name` = the chapter folder (s1c3) – the schematic's path in the references */
export function gapPrompt(sb: StoryboardDoc, shot: Shot, name: string): GapPrompt {
	const cast = Object.fromEntries((sb.cast ?? []).map((c) => [c.ref, c]));
	const places = Object.fromEntries((sb.locations ?? []).map((l) => [l.ref, l]));
	const a = parseAspect(shot.aspect) ?? { w: 9, h: 16 };
	const loc = shot.location !== undefined ? places[shot.location] : undefined;
	const who = (shot.figures ?? []).map((f) => {
		const c = cast[f.cast];
		const persona = f.persona ?? c?.persona;
		const desc = c?.new ? c.new : PERSONA[`${c?.character}/${persona}`] ?? c?.character ?? f.cast;
		const x = f.x as number, y = f.y as number; // undefined compares false: 'centre', 'middle'
		const where = `${x < 0.4 ? 'left' : x > 0.6 ? 'right' : 'centre'}, ${y < 0.35 ? 'upper' : y > 0.65 ? 'lower' : 'middle'} part of the frame`;
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
	const framing = [get(SIZE, shot.shot?.size), get(ANGLE, shot.shot?.angle), get(PROJ, shot.projection)].filter(Boolean).join(', ');
	const air = airPhrase(shot.air);
	const prompt = [
		format,
		shot.prompt?.en ?? '',
		get(VIEW_LINE, shot.view) ?? '',
		framing ? framing[0]!.toUpperCase() + framing.slice(1) + '.' : '',
		loc ? `Location: ${loc.place ?? loc.new ?? loc.ref}${loc.time && loc.time !== 'interior' ? `, ${loc.time}` : ''}.` : '',
		who.length ? `Figures: ${who.join('; ')}.` : 'No people.',
		lightPhrase({ ...(sb.light ?? {}), ...(shot.light ?? {}) }),
		air ? `Composition: ${air}.` : '',
		STYLE[sb.toon] ?? ''
	].filter(Boolean).join(' ');
	const refs = [
		...(shot.figures ?? [])
			.map((f) => cast[f.cast] && { ...cast[f.cast]!, persona: f.persona ?? cast[f.cast]!.persona, look: f.look ?? cast[f.cast]!.look })
			.filter((c) => !!c)
			.map((c) => (c.new ? `${c.new} (no reference yet – Wanted)` : `${c.character} / ${c.persona}${c.look && c.look !== 'default' ? ` (${c.look})` : ''}: the base views + a mood portrait`)),
		loc ? `${loc.place ?? loc.ref}: the Figma selection on the chapter page (closest photo of the place)` : null,
		'the schematic as "layout only" (storyboards/_out/' + name + '/png/' + shot.id + '.png)'
	].filter((r): r is string => !!r);
	return { prompt, check, aspect: shot.format === 'loop360' ? `${a.w}:${a.h} panorama` : shot.aspect, refs: [...new Set(refs)] };
}

/** the Markdown sheet of a chapter's prompts: the gaps (source.want = generated), or every shot with `all` */
export function promptSheet(name: string, scenes: StoryboardDoc[], all = false): { markdown: string; count: number } {
	const md = [`# Prompts ${name} – ${all ? 'all shots' : 'the gaps (source.want = generated)'}`, '',
		'Manual prompting (Luma). Paste the prompt, set the aspect, attach the references. German = what the shot shows (storyboard); the prompt is English. Text and lettering are never in the image – they are live text boxes.', ''];
	let n = 0;
	for (const sb of scenes) {
		for (const { shot } of shotsOf(sb)) {
			if (!all && shot.source?.want !== 'generated') continue;
			n++;
			const g = gapPrompt(sb, shot, name);
			md.push(`## ${shot.id} – ${shot.format} ${shot.aspect}`, '', `**Shows (storyboard):** ${textOf(shot.action)}`, '',
				'```text', g.prompt, '```', '', `**Aspect:** ${g.aspect} · **safe format:** ${g.check}`, '',
				`**Attach:** ${g.refs.join(' · ')}`, '', ...(shot.prompt?.en ? [] : ['**No `prompt.en` yet:** the German action holds the details – add them in English before prompting.', '']));
		}
	}
	return { markdown: md.join('\n'), count: n };
}
