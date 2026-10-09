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

/** { file (absolute), rel (relative to media/), mood } or null */
export function characterImage(c, framing, mood) {
	const media = mediaRoot();
	if (!media || !c?.character) return null;
	const folder = !c.persona || c.persona === 'regular' ? c.character : `${c.character}-${c.persona}`;
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
