// The references for a shot (from scripts/storyboard/refs.mjs, #119) as pure logic: the character image per figure
// (the persona's reference set by framing, one picked per mood so shots vary; else the media images by view and mood).
// No file system: the caller says which files exist (the old tool listed storyboards/style/refs/ and media/; online
// it is the asset manager / the registry's images). Paths out are relative: refs/<char>/<persona>/<look>/<file>
// (to storyboards/style) and characters/<folder>/<folder>_<view>-<mood>.<ext> (to media/).

import type { CastEntry, Figure, StoryboardDoc } from './types.ts';

/** the registry's moods (characters / moods entity) – the character images exist for these */
export const MOODS = ['acceptance', 'angry', 'bargaining', 'confused', 'crying', 'denial', 'disgusted', 'dizzy', 'embarrassed', 'furious', 'happy', 'laughing', 'neutral', 'sad', 'scared', 'smug', 'surprised', 'thinking'] as const;
/** free storyboard moods → the nearest registry mood (until storyboards use the registry's words) */
const NEAR: Record<string, string> = {
	terrified: 'scared', panicked: 'scared', uneasy: 'confused', curious: 'confused', dazed: 'dizzy', clumsy: 'dizzy', falling: 'dizzy',
	exhausted: 'sad', cold: 'sad', nostalgic: 'sad', desperate: 'crying', despairing: 'crying',
	determined: 'smug', driven: 'smug', knowing: 'smug', focused: 'thinking', reverent: 'thinking',
	fierce: 'furious', hunting: 'furious', charging: 'furious', menacing: 'furious', impatient: 'angry', frustrated: 'angry', urgent: 'angry',
	awe: 'surprised', entranced: 'surprised', excited: 'happy', thrilled: 'happy', ecstatic: 'laughing', delighted: 'happy', eager: 'happy', playful: 'happy',
	relieved: 'acceptance', hopeful: 'acceptance', mocking: 'laughing'
};
/** a free mood → a registry mood ('neutral' when nothing is near) */
export const moodOf = (m: string | undefined): string => ((MOODS as readonly unknown[]).includes(m) ? m! : (m !== undefined ? NEAR[m] : undefined) ?? 'neutral');

/** what exists: the files of a reference folder ('catpope/human/default' → names, null = no folder), a media path */
export interface RefSource {
	refFiles(id: string): string[] | null;
	mediaHas?(rel: string): boolean;
}

export interface CharacterImage {
	/** relative: refs/… (storyboards/style) or characters/… (media) */
	rel: string;
	/** the registry mood the image was chosen for */
	mood: string;
}

type CastLike = Partial<Pick<CastEntry, 'character' | 'persona' | 'look'>>;

/** the reference set of a framing: a face for close-ups, a half figure for medium close-ups, else the full figure */
export const refSet = (framing: string | undefined): 'face' | 'half' | 'full' =>
	['cu', 'ecu'].includes(framing ?? '') ? 'face' : ['mcu', 'ms', 'mws'].includes(framing ?? '') ? 'half' : 'full';

/** a persona's generation reference (owner 2026-10-10: refs/<character>/<persona>/<look>/ face-* / half-* / full-*) */
export function referenceImage(c: CastLike, framing: string | undefined, mood: string | undefined, src: RefSource): CharacterImage | null {
	const id = `${c.character}/${c.persona ?? 'regular'}/${c.look ?? 'default'}`;
	const all = src.refFiles(id);
	if (!all) return null;
	const set = refSet(framing);
	const files = all.filter((f) => f.startsWith(`${set}-`) && /\.(jpe?g|png|webp)$/i.test(f)).sort();
	if (!files.length) return null;
	const m = moodOf(mood);
	const f = files[[...m].reduce((h, ch) => h + ch.charCodeAt(0), 0) % files.length]!;
	return { rel: `refs/${id}/${f}`, mood: m };
}

/** the character image of a cast entry: its reference set first, else the media images (view by framing, mood, neutral) */
export function characterImage(c: CastLike | null | undefined, framing: string | undefined, mood: string | undefined, src: RefSource): CharacterImage | null {
	if (!c?.character) return null;
	const folder = !c.persona || c.persona === 'regular' ? c.character : `${c.character}-${c.persona}`;
	const ref = referenceImage(c, framing, mood, src);
	if (ref) return ref;
	if (!src.mediaHas) return null;
	const views = ['cu', 'ecu', 'mcu'].includes(framing ?? '') ? ['portrait', 'avatar', 'full'] : ['full', 'portrait'];
	for (const view of views) for (const m of [moodOf(mood), 'neutral']) for (const ext of ['webp', 'png']) {
		const rel = `characters/${folder}/${folder}_${view}-${m}.${ext}`;
		if (src.mediaHas(rel)) return { rel, mood: m };
	}
	return null;
}

/** the figures of a shot with their cast (persona override applied) and character image */
export function shotCharacters(sb: Pick<StoryboardDoc, 'cast'>, shot: { figures?: Figure[] }, src: RefSource): { fig: Figure; cast: CastLike & { ref?: string }; image: CharacterImage | null }[] {
	const cast = Object.fromEntries((sb.cast ?? []).map((c) => [c.ref, c]));
	return (shot.figures ?? []).map((fig) => {
		const c = { ...(cast[fig.cast] ?? {}), ...(fig.persona ? { persona: fig.persona } : {}) };
		return { fig, cast: c, image: characterImage(c, fig.framing, fig.mood, src) };
	});
}
