// The references for a shot (refpack.mjs, the studio): the character image per figure (persona folder, view by
// framing, the shot's mood mapped to the registry's moods) and the style pictures. From the media dev copy –
// production has the truth (the SCUTOONS MCP reads it).

import { existsSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { ROOT, mediaRoot } from './lib.mjs';

/** the registry's moods (characters / moods entity) – the character images exist for these */
export const MOODS = ['acceptance', 'angry', 'bargaining', 'confused', 'crying', 'denial', 'disgusted', 'dizzy', 'embarrassed', 'furious', 'happy', 'laughing', 'neutral', 'sad', 'scared', 'smug', 'surprised', 'thinking'];
/** free storyboard moods → the nearest registry mood (until storyboards use the registry's words) */
const NEAR = {
	terrified: 'scared', panicked: 'scared', uneasy: 'confused', curious: 'confused', dazed: 'dizzy', clumsy: 'dizzy', falling: 'dizzy',
	exhausted: 'sad', cold: 'sad', nostalgic: 'sad', desperate: 'crying', despairing: 'crying',
	determined: 'smug', driven: 'smug', knowing: 'smug', focused: 'thinking', reverent: 'thinking',
	fierce: 'furious', hunting: 'furious', charging: 'furious', menacing: 'furious', impatient: 'angry', frustrated: 'angry', urgent: 'angry',
	awe: 'surprised', entranced: 'surprised', excited: 'happy', thrilled: 'happy', ecstatic: 'laughing', delighted: 'happy', eager: 'happy', playful: 'happy',
	relieved: 'acceptance', hopeful: 'acceptance', mocking: 'laughing'
};
export const moodOf = (m) => (MOODS.includes(m) ? m : NEAR[m] ?? 'neutral');

export const STYLE_DIR = resolve(ROOT, 'storyboards/style');
export const styleFiles = () => (existsSync(STYLE_DIR) ? readdirSync(STYLE_DIR).filter((f) => /\.(png|jpe?g|webp)$/i.test(f)) : []);

/** a persona's generation references (owner 2026-10-10: the human PLF from the photo shootings, Figma SCENE 1–8 –
 *  the media images of catpope-human are the silicone-mask look 'silicon'): storyboards/style/refs/<character>/
 *  <persona>/<look>/ (the creator's ids) face-* / half-* / full-*.jpg, the set by framing, one per mood so shots vary */
const REFS_DIR = resolve(ROOT, 'storyboards/style/refs');
function referenceImage(c, framing, mood) {
	const id = `${c.character}/${c.persona ?? 'regular'}/${c.look ?? 'default'}`;
	const dir = resolve(REFS_DIR, id);
	if (!existsSync(dir)) return null;
	const set = ['cu', 'ecu'].includes(framing) ? 'face' : ['mcu', 'ms', 'mws'].includes(framing) ? 'half' : 'full';
	const files = readdirSync(dir).filter((f) => f.startsWith(`${set}-`) && /\.(jpe?g|png|webp)$/i.test(f)).sort();
	if (!files.length) return null;
	const m = moodOf(mood);
	const f = files[[...m].reduce((h, ch) => h + ch.charCodeAt(0), 0) % files.length];
	return { file: resolve(dir, f), rel: `refs/${id}/${f}`, mood: m };
}

/** { file (absolute), rel (relative to media/), mood } or null */
export function characterImage(c, framing, mood) {
	if (!c?.character) return null;
	const folder = !c.persona || c.persona === 'regular' ? c.character : `${c.character}-${c.persona}`;
	const ref = referenceImage(c, framing, mood);
	if (ref) return ref;
	const media = mediaRoot();
	if (!media) return null;
	const views = ['cu', 'ecu', 'mcu'].includes(framing) ? ['portrait', 'avatar', 'full'] : ['full', 'portrait'];
	for (const view of views) for (const m of [moodOf(mood), 'neutral']) for (const ext of ['webp', 'png']) {
		const rel = `characters/${folder}/${folder}_${view}-${m}.${ext}`;
		const f = resolve(media, rel);
		if (existsSync(f)) return { file: f, rel, mood: m };
	}
	return null;
}

/** the figures of a shot with their character image (persona / look overrides applied) */
export function shotCharacters(sb, shot) {
	const cast = Object.fromEntries((sb.cast ?? []).map((c) => [c.ref, c]));
	return (shot.figures ?? []).map((fig) => {
		const c = { ...(cast[fig.cast] ?? {}), ...(fig.persona ? { persona: fig.persona } : {}) };
		return { fig, cast: c, image: characterImage(c, fig.framing, fig.mood) };
	});
}
