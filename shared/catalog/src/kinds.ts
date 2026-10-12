// The catalog kinds the admin edits (agents/CATALOG.md part 3): one definition per kind drives the shared editor
// (catalog-editor.svelte) – tab label, list columns, a new entry's data, which keys the form shows (the rest is the
// "Advanced" JSON), and what the kind allows (create, delete, reorder, a confirmation before saving).
import type { CatalogEntry, CatalogKind } from '@scutoons/api';
import { catalogLocale as getLocale } from './setup.ts';
import { m } from './messages.ts';

export type Data = Record<string, any>;

/** the kinds with a tab, in the tab order (featured stays on the Homepage page – one place for it) */
export const EDITED_KINDS = ['items', 'recipes', 'albums', 'products', 'editions', 'toons', 'permissions', 'code-types', 'meta'] as const;
export type EditedKind = (typeof EDITED_KINDS)[number];

/** the lists of every kind, read by the forms for their selects (items, editions, products …) */
export type CatalogLists = Partial<Record<CatalogKind, CatalogEntry[]>>;

/** what every kind's form component gets (catalog-editor.svelte) */
export type CatalogFormProps = {
	data: Data;
	/** a new entry (not stored yet) */
	creating: boolean;
	/** the entry's id ('' while creating: the editor's id field) */
	id: string;
	lists: CatalogLists;
	/** the last save's 422 problems (a form shows the ones naming its fields at them) */
	problems: string[];
};

export interface ListColumn {
	id: string;
	label: () => string;
	secondary?: boolean;
	/** the cell as text */
	text?: (data: any, lists: CatalogLists, entry: CatalogEntry) => string;
	/** the cell as a picture (CDN-root path) */
	image?: (data: any, lists: CatalogLists) => string | null | undefined;
}

export interface KindDef {
	kind: EditedKind;
	label: () => string;
	/** one entry, e.g. "Item" (the New button, dialogs) */
	singular: () => string;
	columns: ListColumn[];
	/** the entry's title in the detail */
	title: (data: any, id: string) => string;
	/** a new entry's data (the id is set on save) */
	blank: () => Data;
	/** the keys the form shows; the other keys are edited as JSON ("Advanced"); null = no Advanced */
	formKeys: string[] | null;
	/** the data as the form edits it (an object) and back (permissions: the data is the description string) */
	toForm?: (data: any) => Data;
	fromForm?: (form: Data) => unknown;
	/** the id goes into `data.id` too (server: data_id) */
	dataId: boolean;
	/** the id rule of a new entry (the server's pattern) */
	idPattern: RegExp;
	creatable: boolean;
	deletable: boolean;
	reorderable: boolean;
	/** security-relevant: ask before saving (code types) */
	confirmSave?: boolean;
}

/** a { de, en } text (or a plain string) in the UI language */
export function text(value: unknown): string {
	if (value && typeof value === 'object') {
		const localized = value as Record<string, string>;
		return localized[getLocale()] || localized.en || localized.de || Object.values(localized).find(Boolean) || '';
	}
	return value === null || value === undefined ? '' : String(value);
}

const dash = (value: unknown) => text(value) || '—';
const ID = /^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$/;
const TOON_ID = /^[a-z0-9_-]{1,64}$/;

export const KINDS: Record<EditedKind, KindDef> = {
	items: {
		kind: 'items',
		label: () => m['catalog.items'](),
		singular: () => m['catalog.item'](),
		columns: [
			{ id: 'icon', label: () => '', image: (d) => d.icon },
			{ id: 'id', label: () => 'id', text: (_d, _l, e) => e.id },
			{ id: 'name', label: () => m['catalog.name'](), text: (d) => dash(d.name) },
			{ id: 'toon', label: () => m['catalog.toon'](), secondary: true, text: (d) => dash(d.toon) },
			{ id: 'category', label: () => m['catalog.category'](), text: (d) => `${dash(d.category)} · ${d.kind ?? ''}` },
			{ id: 'slot', label: () => m['catalog.slot'](), secondary: true, text: (d) => dash(d.slot) },
			{ id: 'variants', label: () => m['catalog.variants'](), secondary: true, text: (d) => (Array.isArray(d.variants) && d.variants.length ? String(d.variants.length) : '—') }
		],
		title: (d, id) => text(d.name) || id,
		blank: () => ({
			toon: null,
			name: { de: '', en: '' },
			description: { de: '', en: '' },
			kind: 'virtual',
			category: 'collectible',
			icon: null,
			stackable: false,
			starter: false,
			grantable: [],
			slot: null,
			skills: [],
			edition_id: null,
			product: null,
			variants: null
		}),
		formKeys: ['id', 'toon', 'name', 'description', 'kind', 'category', 'icon', 'stackable', 'starter', 'slot', 'edition_id', 'product', 'album', 'format', 'variants', 'respawn'],
		dataId: true,
		idPattern: ID,
		creatable: true,
		deletable: true,
		reorderable: true
	},
	recipes: {
		kind: 'recipes',
		label: () => m['catalog.recipes'](),
		singular: () => m['catalog.recipe'](),
		columns: [
			// its own picture, else the output item's – what readers see (owner 2026-10-04: "easier to find")
			{ id: 'image', label: () => '', image: (d, lists) => d.image || lists.items?.find((i) => i.id === d.output?.item)?.data?.icon },
			{ id: 'id', label: () => 'id', text: (_d, _l, e) => e.id },
			{
				id: 'recipe',
				label: () => m['catalog.recipe'](),
				text: (d) => `${(d.inputs ?? []).map((i: Data) => `${i.qty} × ${i.item}`).join(' + ')} → ${d.output?.qty ?? '?'} × ${d.output?.item ?? '?'}`
			}
		],
		title: (_d, id) => id,
		blank: () => ({ inputs: [{ item: '', qty: 1 }], output: { item: '', qty: 1 } }),
		formKeys: null,
		dataId: true,
		idPattern: ID,
		creatable: true,
		deletable: true,
		reorderable: true
	},
	albums: {
		kind: 'albums',
		label: () => m['catalog.albums'](),
		singular: () => m['catalog.album'](),
		columns: [
			{ id: 'cover', label: () => '', image: (d) => d.cover },
			{ id: 'id', label: () => 'id', text: (_d, _l, e) => e.id },
			{ id: 'title', label: () => m['catalog.title_field'](), text: (d) => dash(d.title) },
			{ id: 'tracks', label: () => m['catalog.album_tracks'](), text: (d) => String((d.tracks ?? []).length) },
			{
				id: 'carriers',
				label: () => m['catalog.album_carriers'](),
				secondary: true,
				text: (_d, lists, e) =>
					(lists.items ?? [])
						.filter((i) => i.data.album === e.id)
						.map((i) => `${i.id} (${i.data.format})`)
						.join(', ') || '—'
			}
		],
		title: (d, id) => text(d.title) || id,
		blank: () => ({ toon: 'catpope', title: { de: '', en: '' }, tracks: [], access: { download: 'members', listen: 'owners' } }),
		formKeys: ['id', 'toon', 'title', 'cover', 'download', 'tracks', 'access'],
		dataId: true,
		idPattern: /^[a-z0-9][a-z0-9_]{0,63}$/,
		creatable: true,
		deletable: true,
		reorderable: true
	},
	products: {
		kind: 'products',
		label: () => m['catalog.products'](),
		singular: () => m['catalog.product'](),
		columns: [
			{ id: 'icon', label: () => '', image: (d) => d.icon },
			{ id: 'id', label: () => 'id', text: (_d, _l, e) => e.id },
			{ id: 'name', label: () => m['catalog.name'](), text: (d) => dash(d.name) },
			{ id: 'item_id', label: () => m['catalog.item'](), secondary: true, text: (d) => dash(d.item_id) },
			{ id: 'price', label: () => m['catalog.price'](), text: (d) => (typeof d.price === 'number' ? String(d.price) : '—') },
			{ id: 'remaining', label: () => m['catalog.remaining'](), secondary: true, text: (d) => `${d.remaining ?? '—'} / ${d.limited ?? '—'}` }
		],
		title: (d, id) => text(d.name) || id,
		blank: () => ({
			toon: 'catpope',
			item_id: null,
			edition_id: null,
			edition_size: null,
			name: '',
			category: '',
			link: '',
			price: null,
			limited: null,
			remaining: null,
			sizes: [],
			soldOutSizes: [],
			credits: [],
			icon: null,
			defaultVariant: null,
			variants: []
		}),
		formKeys: ['id', 'toon', 'item_id', 'edition_id', 'edition_size', 'name', 'category', 'link', 'price', 'limited', 'remaining', 'sizes', 'soldOutSizes', 'credits', 'icon', 'defaultVariant', 'variants'],
		dataId: true,
		idPattern: ID,
		creatable: true,
		deletable: true,
		reorderable: true
	},
	editions: {
		kind: 'editions',
		label: () => m['catalog.editions'](),
		singular: () => m['catalog.edition'](),
		columns: [
			{ id: 'id', label: () => 'id', text: (_d, _l, e) => e.id },
			{ id: 'name', label: () => m['catalog.name'](), text: (d) => dash(d.name) },
			{ id: 'toon', label: () => m['catalog.toon'](), secondary: true, text: (d) => dash(d.toon) },
			{ id: 'edition_count', label: () => m['catalog.edition_count'](), text: (d) => dash(d.edition_count) },
			{ id: 'variants', label: () => m['catalog.variants'](), secondary: true, text: (d) => (d.variants ?? []).map((v: Data) => v.name).join(', ') || '—' }
		],
		title: (d, id) => text(d.name) || id,
		blank: () => ({ toon: 'catpope', name: '', description: '', edition_number: 1, edition_count: 1, excluded_number: 0, creators: '', variants: [] }),
		formKeys: ['id', 'toon', 'name', 'description', 'edition_number', 'edition_count', 'excluded_number', 'created_at', 'creators', 'variants'],
		dataId: true,
		idPattern: /^[A-Za-z0-9][A-Za-z0-9_-]{0,99}$/,
		creatable: true,
		deletable: true,
		reorderable: true
	},
	toons: {
		kind: 'toons',
		label: () => m['catalog.toons'](),
		singular: () => m['catalog.toon'](),
		columns: [
			{ id: 'avatar', label: () => '' },
			{ id: 'id', label: () => 'id', text: (_d, _l, e) => e.id },
			{ id: 'name', label: () => m['catalog.name'](), text: (d) => dash(d.name) },
			{ id: 'redirectUrl', label: () => m['catalog.redirect_url'](), secondary: true, text: (d) => dash(d.redirectUrl) }
		],
		title: (d, id) => text(d.name) || id,
		blank: () => ({ name: '', redirectUrl: '' }),
		formKeys: null,
		dataId: true,
		idPattern: TOON_ID,
		creatable: true,
		deletable: true,
		reorderable: true
	},
	permissions: {
		kind: 'permissions',
		label: () => m['catalog.permissions'](),
		singular: () => m['catalog.permission'](),
		columns: [
			{ id: 'id', label: () => 'id', text: (_d, _l, e) => e.id },
			{ id: 'description', label: () => m['catalog.description'](), text: (d) => dash(d) }
		],
		title: (_d, id) => id,
		blank: () => ({ description: '' }),
		formKeys: null,
		toForm: (data) => ({ description: typeof data === 'string' ? data : '' }),
		fromForm: (form) => String(form.description ?? ''),
		dataId: false,
		idPattern: /^[A-Za-z0-9][A-Za-z0-9_:.-]{0,99}$/,
		creatable: true,
		deletable: true,
		reorderable: true
	},
	'code-types': {
		kind: 'code-types',
		label: () => m['catalog.types'](),
		singular: () => m['catalog.code_type'](),
		columns: [
			{ id: 'id', label: () => m['codes.col_type'](), text: (_d, _l, e) => e.id },
			{ id: 'claimable', label: () => m['catalog.claimable'](), text: (d) => (d.claimable ? m['common.yes']() : m['common.no']()) },
			{ id: 'signup', label: () => m['catalog.signup'](), text: (d) => (d.signup ? m['common.yes']() : m['common.no']()) },
			{ id: 'grants', label: () => m['catalog.grants'](), text: (d) => (d.grants ?? []).join(', ') || '—' }
		],
		title: (_d, id) => id,
		blank: () => ({ claimable: false, signup: false, grants: [] }),
		formKeys: null,
		dataId: false,
		idPattern: ID,
		creatable: true,
		deletable: true,
		reorderable: true,
		confirmSave: true
	},
	meta: {
		kind: 'meta',
		label: () => m['catalog.meta_tab'](),
		singular: () => m['catalog.meta_entry'](),
		columns: [
			{ id: 'id', label: () => 'id', text: (_d, _l, e) => e.id },
			{ id: 'version', label: () => m['catalog.version'](), text: (d) => dash(d.version) },
			{ id: 'currency', label: () => m['catalog.currency'](), text: (d) => dash(d.currency) },
			{ id: 'checked_at', label: () => m['catalog.checked_at'](), secondary: true, text: (d) => dash(d.checked_at) }
		],
		title: (_d, id) => id,
		blank: () => ({ version: 1 }),
		formKeys: null,
		dataId: false,
		idPattern: ID,
		creatable: false,
		deletable: false,
		reorderable: false
	}
};

/** the problems (422 invalid_catalog) that name one of the words – shown at that field */
export function problemFor(problems: string[], ...words: string[]): string | null {
	const hits = problems.filter((p) => words.some((w) => new RegExp(`(^|[^A-Za-z0-9_])${w.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}([^A-Za-z0-9_]|$)`).test(p)));
	return hits.length ? hits.join(' · ') : null;
}

/** the keys of `data` the form does not show (the Advanced JSON) */
export function restOf(data: Data, formKeys: string[]): Data {
	return Object.fromEntries(Object.entries(data).filter(([key]) => !formKeys.includes(key)));
}

/** `data` with the Advanced part replaced: the form's keys stay (in their place), the rest = `rest` */
export function withRest(data: Data, formKeys: string[], rest: Data): Data {
	const out: Data = {};
	for (const [key, value] of Object.entries(data)) {
		if (formKeys.includes(key)) out[key] = value;
		else if (key in rest) out[key] = rest[key];
	}
	for (const [key, value] of Object.entries(rest)) if (!(key in out) && !formKeys.includes(key)) out[key] = value;
	return out;
}

/** "M, L, XL" ↔ ["M", "L", "XL"] */
export const listText = (list: unknown) => (Array.isArray(list) ? list.join(', ') : '');
export const parseList = (value: string) =>
	value
		.split(',')
		.map((s) => s.trim())
		.filter(Boolean);

/** a number input's value: '' → null */
export const numberOrNull = (value: string): number | null => (value.trim() === '' || Number.isNaN(Number(value)) ? null : Number(value));
