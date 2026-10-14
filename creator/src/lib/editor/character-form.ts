// The character page's pure / store helpers (COMPONENTS.md step 9: character-detail in sections): colours for the
// native picker, empty texts, story points, a look's A-pose, closing answered image requests.
import {
	APOSE_VIEW,
	newLook,
	type AssetRequests,
	type CharacterFileSlot,
	type LocaleText,
	type RegistryCharacter,
	type StoryPoint
} from '@scutoons/content';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import { edit } from '$lib/editor/characters.ts';

/** any CSS colour (#hex or a name like 'lightblue') → #rrggbb for the native colour picker */
export function toHex(color: string | undefined): string {
	if (!color) return '#000000';
	if (/^#[0-9a-f]{6}$/i.test(color)) return color.toLowerCase();
	if (typeof document === 'undefined') return '#000000';
	const ctx = document.createElement('canvas').getContext('2d');
	if (!ctx) return '#000000';
	ctx.fillStyle = '#000000';
	ctx.fillStyle = color;
	return /^#[0-9a-f]{6}$/i.test(ctx.fillStyle) ? ctx.fillStyle : '#000000';
}

/** a text per language, or nothing when every language is empty */
export const cleanText = (text: LocaleText) => (Object.values(text).some((t) => t?.trim()) ? text : undefined);

/** a story point with one part changed (toon / season / chapter); no toon = no point */
export function setPoint(p: StoryPoint | undefined, key: 'toon' | 'season' | 'chapter', raw: string): StoryPoint | undefined {
	const next: StoryPoint = { ...(p ?? { toon: '' }) };
	if (key === 'toon') next.toon = raw;
	else if (raw.trim() === '') delete next[key];
	else next[key] = Math.max(1, Math.round(Number(raw)) || 1);
	return next.toon ? next : undefined;
}

/**
 * A look's A-pose file (owner 2026-10-12: per look): a path sets it (an import may name a look the persona lacks – it is
 * created), none clears it (an A-pose without an image and without slots goes).
 */
export function setLookApose(c: RegistryCharacter, persona: string, look: string, path: string | undefined, languages: string[]) {
	const p = c.personas.find((x) => x.id === persona);
	if (!p) return;
	if (path && !p.looks.some((x) => x.id === look)) p.looks.push(newLook(look, { [languages[0] ?? 'en']: look }, !p.looks.length));
	// (read back: the edit works on reactive proxies – a pushed object is not the stored one)
	const l = p.looks.find((x) => x.id === look);
	if (!l) return;
	if (path) l.apose = { ...l.apose, path };
	else if (l.apose) {
		delete l.apose.path;
		if (!l.apose.slots?.length) delete l.apose;
	}
}

/** closes the open requests of character `id` that the given files answer */
export function closeRequests(requestsDoc: ContentDoc, requests: AssetRequests | null, id: string, slots: CharacterFileSlot[]) {
	const answers = (r: AssetRequests['requests'][number]) =>
		r.status === 'open' &&
		r.character === id &&
		slots.some((s) =>
			s.kind === 'apose'
				? r.view === APOSE_VIEW && r.persona === s.persona && r.look === s.look
				: s.kind === 'expression' && r.persona === s.persona && r.look === s.look && r.mood === s.mood && r.view === s.view
		);
	if (!requests?.requests.some(answers)) return;
	edit<AssetRequests>(requestsDoc, (f) =>
		f.requests.forEach((r) => {
			if (answers(r)) {
				r.status = 'done';
				r.done_at = new Date().toISOString();
			}
		})
	);
}
