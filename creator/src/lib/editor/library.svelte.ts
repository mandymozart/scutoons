// The Library's state, one for the whole creator (research/redesign/library.html): the registry's
// assets as families, loaded once; the panel's and the browser's filters; the selection; a blank slot's pick request.
// The rules are @scutoons/media's library modules (tested there); this only holds and loads.
import { createAssetLibrary } from '@scutoons/api';
import {
	contextOf,
	fits,
	groupFamilies,
	toonOfAddress,
	type AssetFamily,
	type LibraryContext,
	type LibraryKind,
	type LibraryOrigin,
	type LibraryScope,
	type LibrarySort,
	type LibraryTodo,
	type SlotFit
} from '@scutoons/media';
import { assetsApi } from '$lib/api.ts';
import { shared } from '$lib/utils/shared.ts';

const assetLibrary = createAssetLibrary(assetsApi);
const VIEW_KEY = 'sc-creator-library-view';

/** a blank slot asking the Library for a file: the slot's name on top, only what fits pickable */
export interface PickRequest extends SlotFit {
	/** what the bar says: "Look tourist · mood happy" */
	label: string;
	/** where Upload puts a new file for this slot (the slot's convention path) – else the context's folder */
	path?: string;
	onpick: (path: string) => void;
}

export type TileSize = 's' | 'm' | 'l';

function readView(): { size: TileSize; list: boolean; sort: LibrarySort } {
	try {
		const v = JSON.parse(localStorage.getItem(VIEW_KEY) ?? 'null');
		return { size: ['s', 'm', 'l'].includes(v?.size) ? v.size : 'm', list: v?.list === true, sort: ['name', 'updated', 'uses', 'size'].includes(v?.sort) ? v.sort : 'name' };
	} catch {
		return { size: 'm', list: false, sort: 'name' };
	}
}

class Library {
	families = $state.raw<AssetFamily[]>([]);
	status = $state<'idle' | 'loading' | 'ready' | 'error'>('idle');
	error = $state('');

	/** the front item's address (the shell's front tab) and toon: the workspace's pre-filter */
	address = $state<string | null>(null);
	toonHint = $state<string | null>(null);
	readonly context: LibraryContext | null = $derived(contextOf(this.address));
	readonly toon: string | null = $derived(toonOfAddress(this.address) ?? this.toonHint);

	scope = $state<LibraryScope>('context');
	kind = $state<LibraryKind | 'all' | 'entities'>('all');
	query = $state('');
	sort = $state<LibrarySort>('name');
	size = $state<TileSize>('m');
	list = $state(false);

	/** the browser's places */
	todo = $state<LibraryTodo | null>(null);
	folder = $state<string | null>(null);
	origin = $state<LibraryOrigin | null>(null);

	selected = $state<string | null>(null);
	/** the family whose versions tray is open in the panel */
	tray = $state<string | null>(null);
	browser = $state(false);
	pick = $state.raw<PickRequest | null>(null);

	constructor() {
		if (typeof localStorage !== 'undefined') Object.assign(this, readView());
	}

	readonly selectedFamily = $derived(this.families.find((f) => f.key === this.selected) ?? null);

	setView(patch: Partial<{ size: TileSize; list: boolean; sort: LibrarySort }>) {
		Object.assign(this, patch);
		try {
			localStorage.setItem(VIEW_KEY, JSON.stringify({ size: this.size, list: this.list, sort: this.sort }));
		} catch {
			// storage blocked: the view holds for this visit
		}
	}

	/** the front item changed: the context pill goes back to the front */
	follow(address: string | null, toon: string | null) {
		if (address === this.address && toon === this.toonHint) return;
		this.address = address;
		this.toonHint = toon;
		this.scope = this.context ? 'context' : this.toon ? 'toon' : 'all';
		this.folder = null;
	}

	async load(force = false) {
		if (this.status === 'loading' || (this.status === 'ready' && !force)) return;
		this.status = 'loading';
		try {
			const { items } = await assetLibrary.all();
			this.families = groupFamilies(items);
			this.status = 'ready';
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
			this.status = 'error';
		}
	}

	detail(id: string) {
		return assetLibrary.detail(id);
	}

	/** can Use hand this family on? Outside a pick anything can be used */
	usable(family: AssetFamily): boolean {
		return fits(family, this.pick);
	}

	/** Use: a pick fills its slot; otherwise the editors that take a drop listen for `sc-library-use` – TODO(#694) */
	use(family: AssetFamily, path = family.main) {
		if (this.pick) {
			if (!fits(family, this.pick)) return;
			const request = this.pick;
			this.pick = null;
			request.onpick(path);
			return;
		}
		window.dispatchEvent(new CustomEvent('sc-library-use', { detail: { path, key: family.key, kind: family.kind } }));
	}

	startPick(request: PickRequest) {
		this.pick = request;
		this.browser = false;
		this.kind = 'all';
		this.query = '';
		this.selected = null;
	}

	cancelPick() {
		this.pick = null;
	}

	/** where a new file goes: the slot's own path, else `<context folder or the selection's>/<file name>` */
	uploadPath(file: File): string | null {
		if (this.pick?.path) return this.pick.path.replace(/\.[^./]+$/, '') + (file.name.match(/\.[^.]+$/)?.[0] ?? '');
		const context = this.context;
		// a chapter's season is not in its address: its folder is where its files already are
		const sibling = context ? this.families.find((f) => new RegExp(context.pattern).test(f.key)) : undefined;
		const folder = context?.folder ?? (sibling ? `${sibling.folder}/` : this.toon ? `${this.toon}/` : null);
		return folder ? `${folder}${file.name.toLowerCase().replace(/[^a-z0-9._-]+/g, '-')}` : null;
	}

	async upload(file: File): Promise<string | null> {
		const path = this.uploadPath(file);
		if (!path) throw new Error('no folder');
		await assetsApi.upload(file, path);
		await this.load(true);
		const key = this.families.find((f) => f.versions.some((v) => v.path === path))?.key ?? null;
		this.selected = key;
		if (this.pick && key) this.use(this.families.find((f) => f.key === key)!);
		return path;
	}

	/** new bytes for the family's source file (the old ones archived as a version, the CDN purged) */
	async replace(family: AssetFamily, file: File) {
		const source = family.versions.find((v) => v.label === 'source') ?? family.versions[0];
		await assetsApi.replaceAt(file, source.path);
		await this.load(true);
	}
}

export const library = shared('creator-library', () => new Library());
