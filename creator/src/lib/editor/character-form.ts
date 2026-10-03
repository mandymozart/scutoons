// The character page's pure / store helpers (COMPONENTS.md step 9: character-detail in sections): colours for the
// native picker, empty texts, story points, the A-pose looks, closing answered image requests.
import {
	APOSE_LOOKS,
	APOSE_VIEW,
	type AposeLook,
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

/** the A-pose looks shown: the stored ones plus the standard looks not stored yet (empty slots) */
export function aposeLooksOf(character: RegistryCharacter | null): AposeLook[] {
	const stored = character?.apose?.looks ?? [];
	const missing = APOSE_LOOKS.filter((l) => !stored.some((x) => x.id === l.id)).map((l) => ({ id: l.id, label: { ...l.label }, ...(l.sensitive ? { sensitive: true } : {}) }));
	return [...stored, ...missing];
}

/** an A-pose look's file (the look is created when it isn't stored; swim is the default) */
export function applyApose(c: RegistryCharacter, target: string, path: string, languages: string[]) {
	c.apose ??= { looks: [] };
	let look = c.apose.looks.find((l) => l.id === target);
	if (!look) {
		const standard = APOSE_LOOKS.find((l) => l.id === target);
		look = { id: target, label: standard ? { ...standard.label } : { [languages[0] ?? 'en']: target }, ...(standard?.sensitive ? { sensitive: true } : {}) };
		c.apose.looks.push(look);
	}
	c.apose.looks.find((l) => l.id === target)!.path = path;
	if (!c.apose.looks.some((l) => l.default)) (c.apose.looks.find((l) => l.id === APOSE_LOOKS[0].id) ?? c.apose.looks[0]).default = true;
}

/** closes the open requests of character `id` that the given files answer */
export function closeRequests(requestsDoc: ContentDoc, requests: AssetRequests | null, id: string, slots: CharacterFileSlot[]) {
	const answers = (r: AssetRequests['requests'][number]) =>
		r.status === 'open' &&
		r.character === id &&
		slots.some((s) =>
			s.kind === 'apose' ? r.view === APOSE_VIEW && r.look === s.look : s.kind === 'expression' && r.persona === s.persona && r.look === s.look && r.mood === s.mood && r.view === s.view
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
