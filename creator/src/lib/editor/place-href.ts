import { catalogDocKey } from '@scutoons/api';
import { parseEntityKey } from '@scutoons/content';
import { editHref, parseEditKey } from './edit-key.ts';

/**
 * The creator page that edits a stored file (the dashboard's lists, the review list): a chapter → its editor, a season,
 * a toon's entry / settings / cast / credits / script → their pages, a catalog entry → its catalog page, a map document → the map,
 * the character registry → Characters, a deck scene → its page in Scenes, a module or a rig → the editor (`/edit/<key>`),
 * a storyboard → its chapter's storyboard page. `toon` is the file's toon when the key does not name it (a review's toon).
 */
export function placeHref(key: string, toon: string | null = null): string {
	if (parseEditKey(key)) return editHref(key);
	const doc = catalogDocKey(key);
	if (doc) return `/catalog/${doc.kind}?id=${encodeURIComponent(doc.id)}`;
	const ref = parseEntityKey(key);
	const t = ref?.toon ?? toon;
	switch (ref?.kind) {
		case 'chapter':
			return `/${t}/chapter/${ref.number}`;
		case 'storyboard':
			return `/${t}/chapter/${ref.number}/storyboard`;
		case 'season':
			return `/${t}/season/${ref.number}`;
		case 'toon':
		case 'toon-settings':
			return `/${t}/toon`;
		case 'cast':
		case 'credits':
			return `/${t}/${ref.kind}`;
		case 'season-script':
			return `/${t}/season/${ref.number}/script`;
		case 'chapter-script':
			return `/${t}/chapter/${ref.number}/script`;
		case 'kanon':
			return `/kanon/${ref.id}`;
		case 'map':
		case 'place':
		case 'route':
		case 'region':
			return '/map';
		case 'characters':
		case 'character':
		case 'moods':
			return '/characters';
		case 'scene':
			return `/scenes/${encodeURIComponent(ref.id ?? '')}`;
		case 'asset-requests':
			return '/characters?tab=wanted';
		default:
			return t ? `/${t}` : '/';
	}
}
