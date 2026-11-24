// The Library's files and their actions, one for the whole creator: the core (deck-catalog `library::shelf` through
// space-web) over the registry's assets – a file's versions, whether it fits a slot, where an upload goes, the selection –
// what the open items use (each editor hands its document's uses in), a blank slot's pick request. What is listed is the
// server's answer (library-shelf.svelte.ts). The rules are the core's and the server's; this holds and loads.
import { createAssetLibrary, type AssetSummary, type LevelKind, type LibraryEntry, type ThingType } from '@scutoons/api';
import { parseEntityKey } from '@scutoons/content';
import type { AssetFamily, LibraryFitAnswer, LibraryUse, SlotFit } from '@scutoons/media';
import { libraryCore, libraryOpener, loadLibraryModule, type LibraryCore, type LibraryModule, type LibraryOpener } from '@scutoons/media/library';
import { assetsApi } from '$lib/api.ts';
import { shared } from '$lib/utils/shared.ts';
import { familiesByPath, familyAddress, pathOf } from '$lib/editor/library-address.ts';

const assetLibrary = createAssetLibrary(assetsApi);
const VIEW_KEY = 'sc-creator-library-view';
/** the space-web module (creator/static/live/, `npm run build:wasm -w shared/live-deck`) */
const WASM_BASE = '/live/';
/** the tiles' width in rem: continuous, remembered */
export const TILE_SIZE = { min: 5, max: 12, step: 0.5, start: 7 } as const;

/** what every slot's pick says: its name on top, what is in it now, what this thing already has (shown first) */
interface PickBase {
	/** what the head says: "character · showMessage", "Look tourist · mood happy" */
	label: string;
	/** what the slot holds now (a file's path, a thing's address): shown chosen */
	value?: string;
	/** the place shown first, before everything else: This item (the default), the season's cast, This toon */
	first?: 'item' | 'cast' | 'toon';
}

/** a slot asking the Library for a file: only what fits shown */
export interface FilePick extends PickBase, SlotFit {
	/** where Upload puts a new file for this slot (the slot's convention path) – else the open item's folder */
	path?: string;
	onpick: (path: string) => void;
	/** things it takes beside the files (a shot's slot: a generator) – their types, what picking one does, a new one */
	things?: { types: readonly ThingType[]; onthing: (picked: PickedThing) => void; create?: { label: (name: string) => string; run: (name: string) => void } };
}

/** a thing picked, as deep as its slot asked: a character's persona (the level's own id) */
export interface PickedThing {
	address: string;
	entry: LibraryEntry | null;
	persona?: string;
	look?: string;
	mood?: string;
}

/** a slot asking the Library for a thing of `types` (a character, an item, a person) */
export interface ThingPick extends PickBase {
	types: readonly ThingType[];
	/** the level a picked thing is stepped into for (a speaker's persona): a press on the thing steps in */
	depth?: LevelKind;
	/** what the slot holds already (a related item, a credited person): left out */
	taken?: readonly string[];
	/** only these entries fit (an item slot: plain items, no recipes) */
	accept?: (entry: LibraryEntry) => boolean;
	/** a new one from the Find text (a person): the words of the button, what it does */
	create?: { label: (name: string) => string; run: (name: string) => void };
	onthing: (picked: PickedThing) => void;
}

export type PickRequest = FilePick | ThingPick;
export const isThingPick = (request: PickRequest | null): request is ThingPick => !!request && 'types' in request;

interface Remembered {
	tile: number;
	list: boolean;
}

function readView(): Remembered {
	const start: Remembered = { tile: TILE_SIZE.start, list: false };
	try {
		const v = JSON.parse(localStorage.getItem(VIEW_KEY) ?? 'null') ?? {};
		const tile = Number(v.tile);
		return { tile: tile >= TILE_SIZE.min && tile <= TILE_SIZE.max ? tile : start.tile, list: v.list === true };
	} catch {
		return start;
	}
}


class Library {
	core = $state.raw<LibraryCore | null>(null);
	status = $state<'idle' | 'loading' | 'ready' | 'error'>('idle');
	error = $state('');
	/** bumped when the registry's files were read again (an upload, a rename …): the list asks the server again */
	changed = $state(0);
	/** bumped when the core's families or context change */
	private version = $state(0);
	private module: Promise<LibraryModule> | null = null;
	/** each registry asset's registered file (its id → path): the path the Library's answer names it by */
	private registered = $state.raw<ReadonlyMap<string, string>>(new Map());

	/** the front item's address (the shell's front tab) and toon: the places This item and This toon */
	address = $state<string | null>(null);
	toonHint = $state<string | null>(null);
	readonly toon: string | null = $derived(this.address ? (parseEntityKey(this.address)?.toon ?? this.toonHint) : this.toonHint);
	/** the uses each open item's editor read from its document, by the item's address */
	private usesBy = $state.raw<Record<string, LibraryUse[]>>({});

	tile = $state<number>(TILE_SIZE.start);
	list = $state(false);

	/** the entries chosen, by address (a file's `<kind>:<path>`, a thing's address) */
	chosen = $state.raw<string[]>([]);
	anchor = $state<string | null>(null);
	/** the details drawer at the panel's bottom */
	details = $state(false);
	pick = $state.raw<PickRequest | null>(null);
	/** a file just uploaded (its address): its tile says so */
	fresh = $state<string | null>(null);
	/** the file of the tile being dragged (a drag's data can only be read on the drop) */
	dragging = $state<string | null>(null);

	constructor() {
		if (typeof localStorage !== 'undefined') Object.assign(this, readView());
	}

	/** the families by the path of each of their files */
	private readonly byPath: Map<string, AssetFamily> = $derived.by(() => {
		void this.version;
		return familiesByPath(this.core?.families(false) ?? []);
	});

	/** the family a file address (`<kind>:<path>`) names – by any of its files */
	familyAt(address: string): AssetFamily | null {
		return this.byPath.get(pathOf(address)) ?? null;
	}

	/** a family's address as the Library's answer names it */
	fileAddress(family: AssetFamily): string {
		return familyAddress(family, this.registered);
	}

	readonly selectedFamily: AssetFamily | null = $derived(this.chosen.length ? this.familyAt(this.chosen[this.chosen.length - 1]) : null);
	readonly chosenFamilies: AssetFamily[] = $derived(this.chosen.map((k) => this.familyAt(k)).filter((f): f is AssetFamily => !!f));

	/** whether the open item has a use index (its editor handed one in); else its folder stands for it */
	readonly indexed: boolean = $derived(!!this.address && this.address in this.usesBy);

	setView(patch: Partial<Remembered>) {
		Object.assign(this, patch);
		try {
			localStorage.setItem(VIEW_KEY, JSON.stringify({ tile: this.tile, list: this.list }));
		} catch {
			// storage blocked: the view holds for this visit
		}
	}

	private sync() {
		if (!this.core) return;
		const uses = Object.values(this.usesBy).flat();
		this.core.setContext({ open: this.address, toon: this.toon, byHome: !this.indexed, uses });
		this.version++;
	}

	/** the front item changed */
	follow(address: string | null, toon: string | null) {
		if (address === this.address && toon === this.toonHint) return;
		this.address = address;
		this.toonHint = toon;
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

	usesOf(key: string): LibraryUse[] {
		void this.version;
		return this.core?.uses(key) ?? [];
	}

	/** the registry's files into the core – once; again only when asked (`force`), never by itself after a failure */
	async load(force = false) {
		if (this.status === 'loading' || ((this.status === 'ready' || this.status === 'error') && !force)) return;
		this.status = 'loading';
		try {
			this.module ??= loadLibraryModule(WASM_BASE);
			const [mod, { items }] = await Promise.all([this.module, assetLibrary.all()]);
			this.core?.free();
			this.core = libraryCore(mod, items as AssetSummary[]);
			this.registered = new Map((items as AssetSummary[]).map((a) => [a.id, a.path]));
			this.status = 'ready';
			this.sync();
			if (force) this.changed++;
		} catch (e) {
			this.module = null;
			this.error = e instanceof Error ? e.message : String(e);
			this.status = 'error';
		}
	}

	/** the core's open rule for an editor (the creator's own by default): where an address opens, or why not (no assets needed) */
	async opener(editor: Parameters<typeof libraryOpener>[1] = 'creator'): Promise<LibraryOpener> {
		this.module ??= loadLibraryModule(WASM_BASE);
		return libraryOpener(await this.module, editor);
	}

	detail(id: string) {
		return assetLibrary.detail(id);
	}

	/** whether a file fits a slot (a picker, a tree row) – and if not, what is wrong */
	takes(path: string, fit: SlotFit): LibraryFitAnswer {
		return this.core?.takes(path, fit) ?? { ok: false, unknown: true };
	}

	/** the family holding `path` */
	familyOf(path: string): AssetFamily | null {
		return this.byPath.get(path) ?? null;
	}

	/** the Library's address of the file holding `path` (a slot may hold one of its levels) */
	addressOf(path: string): string | undefined {
		const family = path ? this.familyOf(path) : null;
		return family ? this.fileAddress(family) : undefined;
	}

	/** a click on a tile: one, Ctrl / ⌘ adds or takes one, Shift a run in `order` (the addresses as shown) */
	click(address: string, keys: { shift?: boolean; toggle?: boolean } = {}, order: string[] = []) {
		if (!this.core) {
			this.chosen = [address];
			this.anchor = address;
		} else {
			const next = this.core.select(this.chosen, this.anchor, address, order, keys);
			this.chosen = next.chosen;
			this.anchor = next.anchor;
		}
		this.details = this.chosen.length > 0 && !!this.selectedFamily;
	}

	clearSelection() {
		this.chosen = [];
		this.anchor = null;
		this.details = false;
	}

	/** Use: the editors that take a drop listen for `sc-library-use` – TODO(#694) */
	use(family: AssetFamily, path = family.main) {
		window.dispatchEvent(new CustomEvent('sc-library-use', { detail: { path, key: family.key, kind: family.kind } }));
	}

	/** a slot asks: the picker opens on the overlay level (library-picker.svelte) */
	startPick(request: PickRequest) {
		this.pick = request;
	}

	cancelPick() {
		this.pick = null;
	}

	/** the file pick's answer: when it fits, its slot is filled and the picker closes – else why not */
	pickFile(path: string): LibraryFitAnswer {
		const request = this.pick;
		if (!request || isThingPick(request)) return { ok: false, unknown: true };
		const answer = this.takes(path, request);
		if (!answer.ok) return answer;
		this.pick = null;
		request.onpick(path);
		return answer;
	}

	/** the thing pick's answer: its slot is filled, the picker closes */
	pickThing(picked: PickedThing) {
		const request = this.pick;
		const onthing = isThingPick(request) ? request.onthing : request?.things?.onthing;
		if (!onthing) return;
		this.pick = null;
		onthing(picked);
	}

	/** where a new file goes: the slot's own path, else the open item's folder */
	uploadPath(file: File): string | null {
		const pick = this.pick && !isThingPick(this.pick) ? this.pick : null;
		return this.core?.uploadPath(file.name, pick?.path) ?? null;
	}

	/** a new file; an upload that fits the picker is picked at once. → its path and, when the image rule converted it, what it became */
	async upload(file: File): Promise<{ path: string; converted: string | null }> {
		const asked = this.uploadPath(file);
		if (!asked) throw new Error('no folder');
		const answer = await assetsApi.upload(file, asked);
		const path = answer.uploaded || asked;
		await this.load(true);
		const family = this.familyOf(path);
		this.fresh = family ? this.fileAddress(family) : null;
		if (family) this.click(this.fileAddress(family));
		if (this.pick && !isThingPick(this.pick)) this.pickFile(path);
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
