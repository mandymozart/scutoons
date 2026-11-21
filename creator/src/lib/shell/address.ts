// The shell's addresses (research/redesign/PLAN.md §2.1): every opened item is a tab named by its `kind:id` address, and
// its kind decides the workspace it opens in. The creator's routes stay the URLs (RULES I1 covers only the reader's),
// so a route reads as an address and an address leads back to a route. Pure: test/shell.test.ts.
import { CATALOG_DOC_KINDS, catalogDocKey, type CatalogDocKind } from '@scutoons/api';
import { parseEntityKey } from '@scutoons/content';
import { editHref, parseEditKey } from '../editor/edit-key.ts';

/** what an opened item is edited in – one background and its default panels each */
export type WorkspaceKind = 'toon' | 'chapter' | 'storyboard' | 'character' | 'catalog' | 'map' | 'scene' | 'flow' | 'sound' | 'text' | 'help';

/** an item's kind → its workspace and the icon its tab carries */
const KINDS: Record<string, { workspace: WorkspaceKind; icon: string }> = {
	toon: { workspace: 'toon', icon: 'book' },
	season: { workspace: 'toon', icon: 'layers' },
	chapter: { workspace: 'chapter', icon: 'columns' },
	storyboard: { workspace: 'storyboard', icon: 'film' },
	character: { workspace: 'character', icon: 'smile' },
	characters: { workspace: 'character', icon: 'users' },
	catalog: { workspace: 'catalog', icon: 'package' },
	map: { workspace: 'map', icon: 'map' },
	scene: { workspace: 'scene', icon: 'box' },
	scenes: { workspace: 'scene', icon: 'box' },
	flow: { workspace: 'flow', icon: 'git-merge' },
	rig: { workspace: 'sound', icon: 'sliders' },
	module: { workspace: 'sound', icon: 'cpu' },
	kanon: { workspace: 'text', icon: 'file-text' },
	guide: { workspace: 'help', icon: 'help-circle' },
	notifications: { workspace: 'help', icon: 'bell' }
};
const CATALOG = { workspace: 'catalog' as const, icon: 'package' };

export const kindOf = (address: string): string => address.slice(0, Math.max(0, address.indexOf(':'))) || address;

/** the workspace an address opens in (an unknown kind: none – it is not a tab) */
export function workspaceOf(address: string): WorkspaceKind | null {
	if (catalogDocKey(address)) return 'catalog';
	return KINDS[kindOf(address)]?.workspace ?? null;
}

export function iconOf(address: string): string {
	if (catalogDocKey(address)) return CATALOG.icon;
	return KINDS[kindOf(address)]?.icon ?? 'file';
}

/** the areas' own addresses (a list, not one item): what their tab is called before an item is picked */
export const AREAS = {
	characters: 'characters:all',
	wanted: 'characters:wanted',
	map: 'map:world',
	scenes: 'scenes:all',
	flow: 'flow:universe',
	guide: 'guide:style',
	notifications: 'notifications:push'
} as const;

const TOON = /^[a-z0-9][a-z0-9_-]*$/;
const RESERVED = new Set(['characters', 'map', 'scenes', 'flow', 'guide', 'notifications', 'catalog', 'edit', 'kanon', 'toons', 'docs', 'h', 'kit']);

/**
 * The address a creator URL edits: one item per tab, so a toon's own pages (its board, script, credits,
 * settings) are the toon's tab, a season's cover and script the season's, a chapter's script the chapter's. Home (`/`,
 * `/toons`) and anything unknown: null.
 */
export function addressOf(pathname: string, search = ''): string | null {
	const params = new URLSearchParams(search);
	const parts = pathname.split('/').filter(Boolean).map((p) => decodeURIComponent(p));
	const [first, second, third, fourth] = parts;
	if (!first || first === 'toons') return null;
	switch (first) {
		case 'characters': {
			const id = params.get('id');
			if (id) return `character:${id}`;
			const tab = params.get('tab');
			return tab === 'wanted' ? AREAS.wanted : AREAS.characters;
		}
		case 'map':
			return AREAS.map;
		case 'scenes':
			return second ? `scene:${second}` : AREAS.scenes;
		case 'flow':
			return AREAS.flow;
		case 'guide':
			return AREAS.guide;
		case 'notifications':
			return AREAS.notifications;
		case 'catalog': {
			if (second === 'items' && third && fourth === 'collage') return `item:${third}`;
			const page = (second && second in CATALOG_DOC_KINDS ? second : 'items') as CatalogDocKind;
			const id = params.get('id');
			return id ? `${CATALOG_DOC_KINDS[page]}:${id}` : `catalog:${page}`;
		}
		case 'edit':
			return second && parseEditKey(second) ? second : null;
		case 'kanon':
			return second ? `kanon:${second}` : null;
	}
	if (RESERVED.has(first) || !TOON.test(first)) return null;
	if (second === 'season' && third) return `season:${first}:${third}`;
	if (second === 'chapter' && third) return fourth === 'storyboard' ? `storyboard:${first}:${third}` : `chapter:${first}:${third}`;
	return `toon:${first}`;
}

/** where an address is edited (its default page); unknown: null */
export function hrefOf(address: string): string | null {
	const [kind, ...rest] = address.split(':');
	const id = rest.join(':');
	if (address === AREAS.characters) return '/characters';
	if (address === AREAS.wanted) return '/characters?tab=wanted';
	if (address === AREAS.map || kind === 'place' || kind === 'route' || kind === 'region') return kind === 'map' ? '/map' : `/map?${kind === 'place' ? 'location' : kind}=${encodeURIComponent(id)}`;
	if (address === AREAS.scenes) return '/scenes';
	if (address === AREAS.flow) return '/flow';
	if (address === AREAS.guide) return '/guide';
	if (address === AREAS.notifications) return '/notifications';
	if (kind === 'catalog' && id) return `/catalog/${encodeURIComponent(id)}`;
	if (parseEditKey(address)) return editHref(address);
	const doc = catalogDocKey(address);
	if (doc) return `/catalog/${doc.kind}?id=${encodeURIComponent(doc.id)}`;
	const ref = parseEntityKey(address);
	switch (ref?.kind) {
		case 'toon':
			return ref.toon ? `/${ref.toon}` : null;
		case 'season':
			return `/${ref.toon}/season/${ref.number}`;
		case 'chapter':
			return `/${ref.toon}/chapter/${ref.number}`;
		case 'storyboard':
			return `/${ref.toon}/chapter/${ref.number}/storyboard`;
		case 'character':
			return `/characters?id=${encodeURIComponent(ref.id ?? '')}`;
		case 'scene':
			return `/scenes/${encodeURIComponent(ref.id ?? '')}`;
		case 'kanon':
			return `/kanon/${ref.id}`;
	}
	return null;
}

/** an address typed by hand (`character:catpope`, `chapter:catpope:3`): its page when it is one */
export function resolveTyped(text: string): string | null {
	const address = text.trim();
	if (!address.includes(':')) return null;
	return workspaceOf(address) || parseEntityKey(address) ? hrefOf(address) : null;
}
