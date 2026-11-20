// The open item's uses of files, read from its document for the Library's core (deck-catalog `library::shelf`): every
// string of the document that names a media file, with the part it sits in, in words, and where a click on it leads.
// A toon's documents name files relative to the toon's folder (@scutoons/content `toonAsset`).
// Pure: test/library-uses.test.ts.
import { toonAsset } from '@scutoons/content';
import type { LibraryUse } from '@scutoons/media';

const MEDIA = /\.(webp|png|jpe?g|gif|svg|avif|mp4|webm|mov|m4v|mp3|ogg|wav|flac|m4a|aac|opus|glb|gltf|vtt|srt)$/i;

/** one step of a pointer: an object's key, or an array's place with the element's id when it has one */
type Step = { key: string } | { index: number; id: string | null };

/** `slices` → `slice` */
const singular = (key: string) => (/ies$/.test(key) ? key.replace(/ies$/, 'y') : /[^s]s$/.test(key) ? key.slice(0, -1) : key);

/** a pointer in words: `groups/1/slices/3/image` → `group 2 · slice 4 · image`, an element with an id by its id */
export function partWords(steps: readonly Step[]): string {
	const words: string[] = [];
	for (let i = 0; i < steps.length; i++) {
		const s = steps[i];
		const next = steps[i + 1];
		if ('key' in s && next && 'index' in next) {
			words.push(`${singular(s.key)} ${next.id ?? next.index + 1}`);
			i++;
		} else if ('key' in s) words.push(s.key);
		else words.push(s.id ?? String(s.index + 1));
	}
	return words.join(' · ');
}

/** where a use leads in its item's page: a chapter's group and slice, a character's persona and look */
export function anchorOf(item: string, steps: readonly Step[]): string {
	const after = (key: string) => {
		const at = steps.findIndex((s) => 'key' in s && s.key === key);
		const next = at >= 0 ? steps[at + 1] : undefined;
		return next && 'index' in next ? next : null;
	};
	const params = new URLSearchParams();
	if (item.startsWith('chapter:')) {
		const group = after('groups');
		const slice = after('slices');
		if (group) params.set('group', String(group.index + 1));
		if (group && slice) params.set('slice', String(slice.index + 1));
	} else if (item.startsWith('character:')) {
		const persona = after('personas');
		const look = after('looks');
		if (persona?.id) params.set('persona', persona.id);
		if (look?.id) params.set('look', look.id);
	}
	return params.toString();
}

/** The uses of files in `doc` by `item`; `toon` = the toon whose folder the document's paths are relative to. */
export function usesIn(item: string, doc: unknown, toon: string | null = null): LibraryUse[] {
	const out: LibraryUse[] = [];
	const seen = new Set<string>();
	const walk = (value: unknown, steps: Step[]) => {
		if (typeof value === 'string') {
			if (!MEDIA.test(value.split(/[?#]/)[0]) || /^([a-z][a-z0-9+.-]*:|\/\/)/i.test(value)) return;
			const path = toonAsset(toon, value);
			const part = partWords(steps);
			if (seen.has(`${path} ${part}`)) return;
			seen.add(`${path} ${part}`);
			out.push({ path, item, part, at: anchorOf(item, steps) });
		} else if (Array.isArray(value)) {
			value.forEach((v, index) => {
				const id = v && typeof v === 'object' && typeof (v as { id?: unknown }).id === 'string' ? (v as { id: string }).id : null;
				walk(v, [...steps, { index, id }]);
			});
		} else if (value && typeof value === 'object') {
			for (const [key, v] of Object.entries(value)) walk(v, [...steps, { key }]);
		}
	};
	walk(doc, []);
	return out;
}
