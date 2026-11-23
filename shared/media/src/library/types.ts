// The Library's data as the core gives it (deck-catalog `library::family` / `library::shelf`, through space-web's
// `Library`): JSON types only, no rule here. Formatting for the page (a duration) is the page's.

export type LibraryKind = 'image' | 'video' | 'audio' | 'model' | 'animation' | 'file';
export type LibraryOrigin = 'uploaded' | 'imported' | 'generated';
export type LibraryScope = 'context' | 'toon' | 'all' | 'unused';
export type LibraryGrouping = 'use' | 'kind' | 'none';
export type LibrarySort = 'name' | 'recent' | 'size' | 'uses';
export const LIBRARY_SCOPES: readonly LibraryScope[] = ['context', 'toon', 'all', 'unused'];
export const LIBRARY_GROUPINGS: readonly LibraryGrouping[] = ['use', 'kind', 'none'];
export const LIBRARY_SORTS: readonly LibrarySort[] = ['name', 'recent', 'size', 'uses'];

/** what a tile shows, by the deck's rule: a picture, a still the stage renders (a model), or its kind */
export type LibraryPreview = { picture: { url: string } } | { still: { key: string } } | 'kind' | Record<string, unknown>;

export interface FamilyVersion {
	label: string;
	path: string;
	ext: string;
	kind: LibraryKind;
	size: number;
	width: number | null;
	height: number | null;
	duration: number | null;
	quality: string | null;
	origin: LibraryOrigin;
	asset: string;
}

export interface AssetFamily {
	key: string;
	name: string;
	folder: string;
	title: string;
	kind: LibraryKind;
	assets: string[];
	versions: FamilyVersion[];
	missing: string[];
	usages: number;
	size: number;
	duration: number | null;
	width: number | null;
	height: number | null;
	origin: LibraryOrigin;
	tags: string[];
	updated: string | null;
	version: number;
	/** null: the registry's answer carries no credits */
	credits: boolean | null;
	preview: LibraryPreview;
	main: string;
	made_for: string[];
}

/** a use of a file by the open item, read from its document */
export interface LibraryUse {
	path: string;
	/** the item's address */
	item: string;
	/** the part in words (`slice 4`, `mood happy`) */
	part: string;
	/** where a click leads in the item */
	at: string;
}

export interface LibraryContext {
	open: string | null;
	toon: string | null;
	/** no use index for this item: its files are the ones in its folder */
	byHome?: boolean;
	uses: LibraryUse[];
	/** (main file, part file) links read from documents: frames */
	links?: [string, string][];
	/** the casts the places offer as a facet: a toon season's (`season:catpope:1`) characters */
	casts?: LibraryCast[];
}

export interface LibraryCast {
	address: string;
	characters: string[];
}

/** what a slot takes */
export interface SlotFit {
	kinds: readonly LibraryKind[];
	square?: boolean;
}

export interface LibraryQuery {
	scope: LibraryScope;
	grouping: LibraryGrouping;
	sort: LibrarySort;
	query: string;
	kind: LibraryKind | null;
	/** `recent`, `credits`, `missing`, `item:<address>` or `cast:<address>` */
	special: string | null;
	origin: LibraryOrigin | null;
	fit: SlotFit | null;
	related?: string | null;
	single: boolean;
	now: number;
}

export interface LibraryCell {
	keys: string[];
	/** a frame's kind (a video and its subtitles, a model and its parts), null = one family */
	frame: LibraryKind | null;
}

export interface LibraryGroup {
	key: string;
	what: 'open' | 'unused' | 'item' | 'toon' | 'other' | 'kind' | 'all';
	/** the item's address or the kind's id */
	item: string | null;
	pinned: boolean;
	folded: boolean;
	count: number;
	cells: LibraryCell[];
}

export interface LibraryView {
	scopes: Record<LibraryScope, number>;
	kinds: [LibraryKind, number][];
	specials: { recent: number; credits: number | null; missing: number; items: [string, number][]; casts: [string, number][] };
	origins: [LibraryOrigin, number][];
	groups: LibraryGroup[];
	order: string[];
	total: number;
}

export type LibraryFitAnswer = { ok: true } | { ok: false; wants: LibraryKind[]; got: LibraryKind; square: boolean } | { ok: false; unknown: true };

/** the editors sharing the Library's open rule */
export type LibraryEditor = 'deck' | 'creator';

/** what opening an address means: the tab address and the workspace (the editor's own name for it), or why not */
export type LibraryOpening = { ok: true; address: string; workspace: string } | { ok: false; why: string };

/** a kind the Library lists (the word before `:`), how a list names it, and where it opens or why not */
export interface LibraryOpenKind {
	kind: string;
	words: string;
	workspace?: string;
	/** how a sentence names the workspace ("the map") */
	opens?: string;
	why?: string;
}

/** a tile's duration: `0:12`, `2:10`, `1:02:05` */
export function formatDuration(seconds: number | null): string {
	if (!seconds || seconds <= 0) return '';
	const s = Math.round(seconds);
	const two = (n: number) => String(n).padStart(2, '0');
	return s >= 3600 ? `${Math.floor(s / 3600)}:${two(Math.floor((s % 3600) / 60))}:${two(s % 60)}` : `${Math.floor(s / 60)}:${two(s % 60)}`;
}

/** 1234567 → "1.2 MB" */
export function bytes(n: number): string {
	if (n < 1024) return `${n} B`;
	const units = ['KB', 'MB', 'GB'];
	let v = n / 1024;
	let i = 0;
	while (v >= 1024 && i < units.length - 1) {
		v /= 1024;
		i++;
	}
	return `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`;
}

/** the picture a preview shows ('' = none) */
export function previewPicture(preview: LibraryPreview): string {
	return typeof preview === 'object' && preview && 'picture' in preview ? (preview.picture as { url: string }).url : '';
}

/** whether the preview is a model's still */
export function previewStill(preview: LibraryPreview): boolean {
	return typeof preview === 'object' && !!preview && 'still' in preview;
}
