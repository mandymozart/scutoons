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
	type WorldMap
} from '@scutoons/content';
import { ContentDoc, SaveConflict, SaveError } from '$lib/editor/doc.svelte.ts';
import { draftSaves } from '$lib/editor/draft-saves.svelte.ts';

const N = '_';

/** A collection's documents as stored (newest or published versions), with each key's newest / published version */
export interface StoredCollection {
	docs: CollectionDocs;
	/** key → { latest, published } */
	versions: Record<string, { latest: number; published: number | null }>;
}

/** Loads every document of a collection (newest or published versions), in creation order */
export async function loadCollection(spec: CollectionSpec, content: 'draft' | 'published' = 'draft'): Promise<StoredCollection> {
	const { entities: list } = await withContent(collectionKinds(spec), content);
	const versions: StoredCollection['versions'] = Object.fromEntries(list.map((e) => [e.key, { latest: e.latest, published: e.published }]));
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
	versions = $state<Record<string, { latest: number; published: number | null }>>({});

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
			.filter(([, v]) => v.published !== v.latest)
			.map(([k]) => k);
	}
	override get unpublished(): string[] {
		return this.unpublishedKeys.length ? [N] : [];
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

/** The world map in the builder (map + place:<id> + route:<id>) */
export class WorldMapDoc extends CollectionDoc {
	constructor() {
		super(MAP_COLLECTION, (docs) => composeWorldMap(docs) as unknown as Record<string, any>);
	}
}

/** The character registry in the editors (characters + character:<id>) */
export class CharacterRegistryDoc extends CollectionDoc {
	constructor() {
		super(CHARACTERS_COLLECTION, (docs) => composeCharacterRegistry(docs) as unknown as Record<string, any>);
	}
}
