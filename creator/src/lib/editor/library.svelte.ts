// The Library's state, one for the whole creator: the core (deck-catalog `library::shelf` through space-web) over the
// registry's assets, what the open items use (each editor hands its document's uses in), the view's choices, the
// selection, the folds the person changed, a blank slot's pick request. The rules are the core's; this holds and loads.
import { createAssetLibrary, type AssetSummary } from '@scutoons/api';
import { parseEntityKey } from '@scutoons/content';
import type { AssetFamily, LibraryCast, LibraryFitAnswer, LibraryGrouping, LibraryKind, LibraryOrigin, LibraryQuery, LibraryScope, LibrarySort, LibraryUse, LibraryView, SlotFit } from '@scutoons/media';
import { libraryCore, loadLibraryModule, type LibraryCore, type LibraryModule } from '@scutoons/media/library';
import { assetsApi } from '$lib/api.ts';
import { shared } from '$lib/utils/shared.ts';

const assetLibrary = createAssetLibrary(assetsApi);
const VIEW_KEY = 'sc-creator-library-view';
/** the space-web module (creator/static/live/, `npm run build:wasm -w shared/live-deck`) */
const WASM_BASE = '/live/';
/** the tiles' width in rem: continuous, remembered */
export const TILE_SIZE = { min: 5, max: 12, step: 0.5, start: 7 } as const;

/** a blank slot asking the Library for a file: the slot's name on top, only what fits shown */
export interface PickRequest extends SlotFit {
	/** what the bar says: "Look tourist · mood happy" */
	label: string;
	/** where Upload puts a new file for this slot (the slot's convention path) – else the open item's folder */
	path?: string;
	onpick: (path: string) => void;
}

interface Remembered {
	tile: number;
	list: boolean;
	sort: LibrarySort;
	grouping: LibraryGrouping;
	single: boolean;
}

function readView(): Remembered {
	const start: Remembered = { tile: TILE_SIZE.start, list: false, sort: 'name', grouping: 'use', single: false };
	try {
		const v = JSON.parse(localStorage.getItem(VIEW_KEY) ?? 'null') ?? {};
		const tile = Number(v.tile);
		return {
			tile: tile >= TILE_SIZE.min && tile <= TILE_SIZE.max ? tile : start.tile,
			list: v.list === true,
			sort: ['name', 'recent', 'size', 'uses'].includes(v.sort) ? v.sort : start.sort,
			grouping: ['use', 'kind', 'none'].includes(v.grouping) ? v.grouping : start.grouping,
			single: v.single === true
		};
	} catch {
		return start;
	}
}

class Library {
	core = $state.raw<LibraryCore | null>(null);
	status = $state<'idle' | 'loading' | 'ready' | 'error'>('idle');
	error = $state('');
	/** bumped when the core's families or context change: the view reads it */
	private version = $state(0);
	private module: Promise<LibraryModule> | null = null;

	/** the front item's address (the shell's front tab) and toon: the workspace's pre-filter */
	address = $state<string | null>(null);
	toonHint = $state<string | null>(null);
	readonly toon: string | null = $derived(this.address ? (parseEntityKey(this.address)?.toon ?? this.toonHint) : this.toonHint);
	/** the uses each open item's editor read from its document, by the item's address */
	private usesBy = $state.raw<Record<string, LibraryUse[]>>({});
	/** the casts an editor handed in (the seasons' characters): the places' cast facet */
	private casts = $state.raw<LibraryCast[]>([]);

	scope = $state<LibraryScope>('context');
	grouping = $state<LibraryGrouping>('use');
	sort = $state<LibrarySort>('name');
	kind = $state<LibraryKind | null>(null);
	query = $state('');
	special = $state<string | null>(null);
	origin = $state<LibraryOrigin | null>(null);
	/** every file on its own (the browser's levels) */
	single = $state(false);
	tile = $state<number>(TILE_SIZE.start);
	list = $state(false);
	/** the groups the person folded or opened, by key (back to the core's start on a new scope or grouping) */
	folds = $state<Record<string, boolean>>({});

	chosen = $state.raw<string[]>([]);
	anchor = $state<string | null>(null);
	/** the details drawer at the panel's bottom */
	details = $state(false);
	browser = $state(false);
	pick = $state.raw<PickRequest | null>(null);
	/** a file just uploaded: its tile says so */
	fresh = $state<string | null>(null);

	constructor() {
		if (typeof localStorage !== 'undefined') Object.assign(this, readView());
	}

	/** the families by key, stacked or single as shown */
	readonly byKey: Map<string, AssetFamily> = $derived.by(() => {
		void this.version;
		return new Map((this.core?.families(this.single) ?? []).map((f) => [f.key, f]));
	});

	readonly fit: SlotFit | null = $derived(this.pick ? { kinds: this.pick.kinds, square: this.pick.square } : null);

	readonly view: LibraryView | null = $derived.by(() => {
		void this.version;
		if (!this.core) return null;
		const q: LibraryQuery = {
			scope: this.scope,
			grouping: this.grouping,
			sort: this.sort,
			query: this.query,
			kind: this.kind,
			special: this.special,
			origin: this.origin,
			fit: this.fit,
			single: this.single,
			now: Date.now()
		};
		return this.core.view(q);
	});

	readonly selectedFamily: AssetFamily | null = $derived(this.chosen.length ? (this.byKey.get(this.chosen[this.chosen.length - 1]) ?? null) : null);
	readonly chosenFamilies: AssetFamily[] = $derived(this.chosen.map((k) => this.byKey.get(k)).filter((f): f is AssetFamily => !!f));

	/** whether the open item has a use index (its editor handed one in); else its folder stands for it */
	readonly indexed: boolean = $derived(!!this.address && this.address in this.usesBy);

	folded(group: { key: string; folded: boolean }): boolean {
		return this.folds[group.key] ?? group.folded;
	}

	toggleFold(group: { key: string; folded: boolean }) {
		this.folds = { ...this.folds, [group.key]: !this.folded(group) };
	}

	setView(patch: Partial<Remembered>) {
		if (patch.grouping && patch.grouping !== this.grouping) this.folds = {};
		Object.assign(this, patch);
		try {
			localStorage.setItem(VIEW_KEY, JSON.stringify({ tile: this.tile, list: this.list, sort: this.sort, grouping: this.grouping, single: this.single }));
		} catch {
			// storage blocked: the view holds for this visit
		}
	}

	setScope(scope: LibraryScope) {
		this.scope = scope;
		this.special = null;
		this.folds = {};
	}

	private sync() {
		if (!this.core) return;
		const uses = Object.values(this.usesBy).flat();
		this.core.setContext({ open: this.address, toon: this.toon, byHome: !this.indexed, uses, casts: this.casts });
		this.version++;
	}

	/** the front item changed: the scope goes back to what it uses */
	follow(address: string | null, toon: string | null) {
		if (address === this.address && toon === this.toonHint) return;
		this.address = address;
		this.toonHint = toon;
		this.scope = address ? 'context' : 'all';
		this.special = null;
		this.folds = {};
		this.sync();
	}

	/** an editor's document changed: the files it uses (none = forget it) */
	setUses(address: string, uses: LibraryUse[] | null) {
		const next = { ...this.usesBy };
		if (uses) next[address] = uses;
		else delete next[address];
		this.usesBy = next;
		this.sync();
	}

	/** the casts the places offer (every toon season's characters, read from the cast documents) */
	setCasts(casts: LibraryCast[]) {
		this.casts = casts;
		this.sync();
	}

	usesOf(key: string): LibraryUse[] {
		void this.version;
		return this.core?.uses(key) ?? [];
	}

	async load(force = false) {
		if (this.status === 'loading' || (this.status === 'ready' && !force)) return;
		this.status = 'loading';
		try {
			this.module ??= loadLibraryModule(WASM_BASE);
			const [mod, { items }] = await Promise.all([this.module, assetLibrary.all()]);
			this.core?.free();
			this.core = libraryCore(mod, items as AssetSummary[]);
			this.status = 'ready';
			this.sync();
		} catch (e) {
			this.module = null;
			this.error = e instanceof Error ? e.message : String(e);
			this.status = 'error';
		}
	}

	detail(id: string) {
		return assetLibrary.detail(id);
	}

	/** whether a file fits a slot (a picker, a tree row) – and if not, what is wrong */
	takes(path: string, fit: SlotFit): LibraryFitAnswer {
		return this.core?.takes(path, fit) ?? { ok: false, unknown: true };
	}

	/** a click on a tile: one, Ctrl / ⌘ adds or takes one, Shift a run in the order shown */
	click(key: string, keys: { shift?: boolean; toggle?: boolean } = {}) {
		if (!this.core) return;
		const next = this.core.select(this.chosen, this.anchor, key, this.view?.order ?? [], keys);
		this.chosen = next.chosen;
		this.anchor = next.anchor;
		this.details = this.chosen.length > 0;
	}

	clearSelection() {
		this.chosen = [];
		this.anchor = null;
		this.details = false;
	}

	/** Use: a pick fills its slot; otherwise the editors that take a drop listen for `sc-library-use` – TODO(#694) */
	use(family: AssetFamily, path = family.main) {
		if (this.pick) {
			if (!this.takes(path, this.pick).ok) return;
			const request = this.pick;
			this.pick = null;
			request.onpick(path);
			return;
		}
		window.dispatchEvent(new CustomEvent('sc-library-use', { detail: { path, key: family.key, kind: family.kind } }));
	}

	/** a picker starts in All with the open item pinned, on the kind it picks, showing only what fits */
	startPick(request: PickRequest) {
		this.pick = request;
		this.browser = false;
		this.scope = 'all';
		this.special = null;
		this.folds = {};
		this.kind = this.core?.startKind(request) ?? (request.kinds.length === 1 ? request.kinds[0] : null);
		this.query = '';
		this.clearSelection();
	}

	cancelPick() {
		this.pick = null;
		this.kind = null;
	}

	/** where a new file goes: the slot's own path, else the open item's folder */
	uploadPath(file: File): string | null {
		return this.core?.uploadPath(file.name, this.pick?.path) ?? null;
	}

	/** a new file; an upload that fits the picker is picked at once. → its path and, when the image rule converted it, what it became */
	async upload(file: File): Promise<{ path: string; converted: string | null }> {
		const asked = this.uploadPath(file);
		if (!asked) throw new Error('no folder');
		const answer = await assetsApi.upload(file, asked);
		const path = answer.uploaded || asked;
		await this.load(true);
		const family = this.core?.families(this.single).find((f) => f.versions.some((v) => v.path === path)) ?? null;
		this.fresh = family?.key ?? null;
		if (family) this.click(family.key);
		if (this.pick && family && this.takes(path, this.pick).ok) this.use(family, path);
		return { path, converted: path !== asked ? path : null };
	}

	/** new bytes for the family's source file (the old ones archived as a version, the CDN purged) */
	async replace(family: AssetFamily, file: File) {
		const source = family.versions.find((v) => v.label === 'source') ?? family.versions[0];
		await assetsApi.replaceAt(file, source.path);
		await this.load(true);
	}

	/** Rename: the name only – every file's URL and every use stay */
	async rename(family: AssetFamily, title: string) {
		await Promise.all(family.assets.map((id) => assetsApi.update(id, { title })));
		await this.load(true);
	}

	/** what deleting would break: the uses that block (published) and the drafts' (an admin may force) */
	async deleteCheck(families: AssetFamily[]) {
		const ids = families.flatMap((f) => f.assets);
		const checks = await Promise.all(ids.map((id) => assetsApi.deleteCheck(id)));
		return { ids, blocking: checks.reduce((n, c) => n + c.blocking, 0), drafts: checks.reduce((n, c) => n + c.drafts, 0), free: checks.every((c) => c.can_delete) };
	}

	async remove(ids: string[], force: boolean) {
		for (const id of ids) await assetsApi.remove(id, force);
		this.clearSelection();
		await this.load(true);
	}
}

export const library = shared('creator-library', () => new Library());
