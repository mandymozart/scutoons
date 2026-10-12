// A document made of many documents in the editor (DRAFT, agents/MAP-DOCUMENTS.md): ONE editable composed document
// (data['_'], like before – every edit, undo step, validation stays as it is), stored as MANY documents (the anchor +
// one per list entry – @scutoons/content collection-docs.ts): the world map (`map`, `place:<id>`, `route:<id>`) and the
// character registry (`characters`, `character:<id>`). Loading reads them all in one request (GET /entities?kinds=…)
// and composes them; saving stores only the documents that changed, in one transaction (POST /entities/batch – all or
// none), each against the version it was loaded at: someone else's newer save of THAT entry is a conflict, of another
// one is not. An entry that left: deleted when it was never published (owner D3), else a tombstone (`removed: true`) –
// its id stays taken (events, casts and readers' save games reference it).
import { ApiError, saveBatch, withContent, type ContentVersion } from '$lib/api.ts';
import {
	CHARACTERS_COLLECTION,
	MAP_COLLECTION,
	applyCollectionChanges,
	collectionChanges,
	collectionDocsFromEntities,
	collectionKinds,
	composeCharacterRegistry,
	composeWorldMap,
	emptyCollectionDocs,
	takenCollectionIds,
	type CharacterRegistry,
	type CollectionDocs,
	type CollectionSpec,
	type CellRun,
	type MapRegion,
	type WorldMap
} from '@scutoons/content';
import { ContentDoc, SaveConflict, SaveError } from '$lib/editor/doc.svelte.ts';
import { draftSaves } from '$lib/editor/draft-saves.svelte.ts';

const N = '_';

/** A collection's documents as stored (newest or published versions), with each key's newest / published version */
export interface StoredCollection {
	docs: CollectionDocs;
	/** key → { latest, published } */
	versions: Record<string, { latest: number; published: number | null; same?: boolean }>;
}

/** Loads every document of a collection (newest or published versions), in creation order */
export async function loadCollection(spec: CollectionSpec, content: 'draft' | 'published' = 'draft'): Promise<StoredCollection> {
	const { entities: list } = await withContent(collectionKinds(spec), content);
	const versions: StoredCollection['versions'] = Object.fromEntries(list.map((e) => [e.key, { latest: e.latest, published: e.published, same: e.same_as_published }]));
	return { docs: collectionDocsFromEntities(spec, list), versions };
}

/** The composed world map (newest or published versions) – for the pages that only read it (event forms, flow, wanted images) */
export async function loadWorldMap(content: 'draft' | 'published' = 'draft'): Promise<WorldMap | null> {
	const { docs } = await loadCollection(MAP_COLLECTION, content);
	return docs.settings ? composeWorldMap(docs) : null;
}

/** The composed character registry (newest or published versions) */
export async function loadCharacterRegistry(content: 'draft' | 'published' = 'draft'): Promise<CharacterRegistry | null> {
	const { docs } = await loadCollection(CHARACTERS_COLLECTION, content);
	return docs.settings || Object.values(docs.parts).some((l) => l.length) ? composeCharacterRegistry(docs) : null;
}

export class CollectionDoc extends ContentDoc {
	/** the documents as stored (tombstones included) – what a save compares against */
	private storedDocs = $state<CollectionDocs>({ settings: null, parts: {} });
	/** key → the newest version this editor knows (its save's `base`) / the published one */
	versions = $state<Record<string, { latest: number; published: number | null; same?: boolean }>>({});
	/**
	 * the entry the page has selected (`place:cathedral`, `character:boy`): the History panel shows its versions (owner
	 * D5: "what is selected gets a history"); null = the anchor's own
	 */
	selectedKey = $state<string | null>(null);

	constructor(
		readonly spec: CollectionSpec,
		private readonly compose: (docs: CollectionDocs) => Record<string, any>
	) {
		super(spec.key, [N], N);
		this.storedDocs = emptyCollectionDocs(spec);
	}

	/** every stored key of the collection (the publish checklist lists each as its own file) */
	override storedKeys(): string[] {
		return Object.keys(this.versions);
	}

	/** the keys with a draft newer than what readers see */
	get unpublishedKeys(): string[] {
		return Object.entries(this.versions)
			.filter(([, v]) => v.published !== v.latest && !v.same)
			.map(([k]) => k);
	}
	override get unpublished(): string[] {
		return this.unpublishedKeys.length ? [N] : [];
	}

	/** the stored key the History panel shows: the selected entry when it is stored, else the anchor */
	get historyKey(): string {
		return this.selectedKey && this.versions[this.selectedKey] ? this.selectedKey : this.spec.key;
	}

	/** an entry's name in the composed document (a locale map or a string), null when it isn't in it */
	entryName(key: string): unknown {
		const [kind, id] = key.split(':');
		const field = this.spec.parts.find((p) => p.kind === kind)?.field;
		const entry = field ? (this.data[N]?.[field] as { id: string; name?: unknown }[] | undefined)?.find((e) => e.id === id) : undefined;
		return entry?.name ?? null;
	}

	/** ids a new entry must not take: every one stored (removed ones too – ids are stable forever once published) */
	takenIds(kind: string): string[] {
		return takenCollectionIds(this.storedDocs, kind);
	}

	override async load() {
		this.loading = true;
		this.error = null;
		try {
			const { docs, versions } = await loadCollection(this.spec, 'draft');
			this.storedDocs = docs;
			this.versions = versions;
			const stored = !!docs.settings || Object.values(docs.parts).some((l) => l.length);
			const anchor = versions[this.spec.key];
			this.changedAt[N] = null;
			this.adopt({ [N]: stored ? this.compose(docs) : null }, { [N]: anchor?.latest ?? null }, { [N]: anchor?.published ?? null }, { [N]: stored ? 'store' : 'missing' });
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		}
		this.loading = false;
	}

	/** Stores the changed documents in one transaction; `force` ignores conflicts (overwrites) */
	override async save(message?: string, force = false): Promise<string[]> {
		const composed = this.data[N] as Record<string, any> | null;
		if (!composed || !this.dirty) return [];
		this.saving = true;
		try {
			const snapshot = $state.snapshot(composed) as Record<string, any>;
			const all = collectionChanges(this.spec, $state.snapshot(this.storedDocs) as CollectionDocs, snapshot);
			// an entry that leaves and was never published is deleted outright (owner D3) – its id is free again; a
			// published one gets its `removed: true` version (a tombstone: readers' save games / casts know the id)
			const neverPublished = (key: string) => (this.versions[key]?.published ?? null) === null;
			const deletes = all.filter((c) => c.kind === 'removed' && neverPublished(c.key)).map((c) => c.key);
			const changes = all.filter((c) => !deletes.includes(c.key));
			if (all.length) {
				let result;
				try {
					result = await saveBatch(
						changes.map((c) => ({ key: c.key, content: c.doc, ...(force ? {} : { base: this.versions[c.key]?.latest ?? null }) })),
						message,
						deletes
					);
				} catch (e) {
					if (e instanceof ApiError && e.status === 409) throw new SaveConflict(String(e.body?.key ?? N), (e.body?.latest as ContentVersion | undefined) ?? null);
					throw new SaveError(String((e instanceof ApiError && e.body?.key) || N), e);
				}
				const versions = { ...this.versions };
				for (const s of result.saved) versions[s.key] = { latest: s.version.id, published: versions[s.key]?.published ?? null };
				for (const key of result.deleted ?? []) delete versions[key];
				this.versions = versions;
				this.storedDocs = applyCollectionChanges(this.spec, this.storedDocs, changes, result.deleted ?? []);
			}
			this.base[N] = this.versions[this.spec.key]?.latest ?? null;
			this.source[N] = 'store';
			this.saved[N] = JSON.stringify(snapshot);
			this.lastSavedAt = Date.now();
			draftSaves.report('');
			this.clearAutosave();
			return [N];
		} finally {
			this.saving = false;
		}
	}

	/** publishing goes through the publish checklist (publish-tree.ts – every entry its own line) */
	override async publish(): Promise<string[]> {
		throw new Error('a collection is published through the publish checklist');
	}
}

/**
 * A region cover's row runs kept OUT of Svelte's deep proxies (an Array subclass – `$state` proxies plain arrays only):
 * a region has thousands of runs, and every read through the proxies – the undo history's JSON.stringify of the whole map
 * on each edit above all – cost seconds per brush stroke (owner 2026-10-12, a ThinkPad: "takes forever"). Stored and
 * sent as a plain JSON array; a new cover is assigned whole (`region.cells = rawRuns(…)`), which the store tracks.
 */
class RunList extends Array<CellRun> {}
export const rawRuns = (runs: readonly CellRun[]): CellRun[] => (runs instanceof RunList ? runs : (RunList.from(runs, (r) => [r[0], r[1], r[2]] as CellRun) as CellRun[]));
const unproxyRegions = (map: Record<string, any> | null | undefined) => {
	for (const region of (map?.regions ?? []) as MapRegion[]) region.cells = rawRuns(region.cells ?? []);
	return map;
};

/** The world map in the builder (map + place:<id> + route:<id> + region:<id>) */
export class WorldMapDoc extends CollectionDoc {
	constructor() {
		super(MAP_COLLECTION, (docs) => unproxyRegions(composeWorldMap(docs) as unknown as Record<string, any>)!);
	}
	// undo / redo bring the map back from JSON: the covers out of the proxies again
	override undo() {
		super.undo();
		unproxyRegions(this.data?.[N]);
	}
	override redo() {
		super.redo();
		unproxyRegions(this.data?.[N]);
	}
}

/** The character registry in the editors (characters + character:<id>) */
export class CharacterRegistryDoc extends CollectionDoc {
	constructor() {
		super(CHARACTERS_COLLECTION, (docs) => composeCharacterRegistry(docs) as unknown as Record<string, any>);
	}
}
