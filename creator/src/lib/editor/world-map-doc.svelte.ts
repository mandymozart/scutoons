// The world map in the builder (DRAFT, agents/MAP-DOCUMENTS.md): ONE editable WorldMap (data['_'], like before – every
// builder edit, undo step, validation and the canvas stay as they are), stored as MANY documents: `map` (settings),
// `place:<id>`, `route:<id>`. Loading reads them all in one request (GET /entities?kinds=…) and composes them; saving
// stores only the documents that changed, in one transaction (POST /entities/batch – all or none), each against the
// version it was loaded at: someone else's newer save of THAT place is a conflict, a save of another place is not.
// A place / route that left the map gets a tombstone (`removed: true`): its id stays taken (events reference it).
import { ApiError, saveBatch, withContent, type ContentVersion } from '$lib/api.ts';
import {
	applyMapDocChanges,
	composeWorldMap,
	entities,
	mapDocChanges,
	parseEntityKey,
	takenMapIds,
	type MapDocs,
	type MapPartKind,
	type MapSettings,
	type PlaceDoc,
	type RouteDoc,
	type WorldMap
} from '@scutoons/content';
import { ContentDoc, SaveConflict, SaveError } from '$lib/editor/doc.svelte.ts';
import { draftSaves } from '$lib/editor/draft-saves.svelte.ts';

const N = '_';
export const MAP_DOC_KINDS = ['map', 'place', 'route'];

/** The world map's documents as stored (newest versions), with each key's newest / published version */
export interface StoredMapDocs {
	docs: MapDocs;
	/** key → { latest, published } */
	versions: Record<string, { latest: number; published: number | null }>;
}

/** Loads every document of the world map (newest or published versions), in creation order */
export async function loadMapDocs(content: 'draft' | 'published' = 'draft'): Promise<StoredMapDocs> {
	const { entities: list } = await withContent(MAP_DOC_KINDS, content);
	const docs: MapDocs = { settings: null, places: [], routes: [] };
	const versions: StoredMapDocs['versions'] = {};
	for (const e of list) {
		versions[e.key] = { latest: e.latest, published: e.published };
		if (e.content === null) continue;
		const ref = parseEntityKey(e.key);
		if (ref?.kind === 'map') docs.settings = e.content as MapSettings;
		else if (ref?.kind === 'place') docs.places.push({ id: ref.id!, doc: e.content as PlaceDoc });
		else if (ref?.kind === 'route') docs.routes.push({ id: ref.id!, doc: e.content as RouteDoc });
	}
	return { docs, versions };
}

/** The composed world map (newest or published versions) – for the pages that only read it (event forms, flow, wanted images) */
export async function loadWorldMap(content: 'draft' | 'published' = 'draft'): Promise<WorldMap | null> {
	const { docs } = await loadMapDocs(content);
	return docs.settings ? composeWorldMap(docs) : null;
}

export class WorldMapDoc extends ContentDoc {
	/** the documents as stored (tombstones included) – what a save compares against */
	private storedDocs = $state<MapDocs>({ settings: null, places: [], routes: [] });
	/** key → the newest version this editor knows (its save's `base`) / the published one */
	versions = $state<Record<string, { latest: number; published: number | null }>>({});

	constructor() {
		super(entities.map, [N], N);
	}

	/** every stored key of the map (the publish checklist lists each as its own file) */
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

	/** ids a new place / route must not take: every one ever stored (removed ones too – ids are stable forever) */
	takenIds(kind: MapPartKind): string[] {
		return takenMapIds(this.storedDocs, kind);
	}
	/** the published / restorable state of a part: a tombstone in the store (removed from the map) */
	isRemoved(kind: MapPartKind, id: string): boolean {
		return (kind === 'place' ? this.storedDocs.places : this.storedDocs.routes).some((p) => p.id === id && !!p.doc.removed);
	}

	override async load() {
		this.loading = true;
		this.error = null;
		try {
			const { docs, versions } = await loadMapDocs('draft');
			this.storedDocs = docs;
			this.versions = versions;
			const stored = !!docs.settings;
			const map = stored ? composeWorldMap(docs) : null;
			const latest = versions[entities.map];
			this.changedAt[N] = null;
			this.adopt({ [N]: map }, { [N]: latest?.latest ?? null }, { [N]: latest?.published ?? null }, { [N]: stored ? 'store' : 'missing' });
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		}
		this.loading = false;
	}

	/** Stores the changed documents in one transaction; `force` ignores conflicts (overwrites) */
	override async save(message?: string, force = false): Promise<string[]> {
		const map = this.data[N] as WorldMap | null;
		if (!map || !this.dirty) return [];
		this.saving = true;
		try {
			const snapshot = $state.snapshot(map) as WorldMap;
			const changes = mapDocChanges($state.snapshot(this.storedDocs) as MapDocs, snapshot);
			if (changes.length) {
				let result;
				try {
					result = await saveBatch(
						changes.map((c) => ({ key: c.key, content: c.doc, ...(force ? {} : { base: this.versions[c.key]?.latest ?? null }) })),
						message
					);
				} catch (e) {
					if (e instanceof ApiError && e.status === 409) throw new SaveConflict(String(e.body?.key ?? N), (e.body?.latest as ContentVersion | undefined) ?? null);
					throw new SaveError(String((e instanceof ApiError && e.body?.key) || N), e);
				}
				const versions = { ...this.versions };
				for (const s of result.saved) versions[s.key] = { latest: s.version.id, published: versions[s.key]?.published ?? null };
				this.versions = versions;
				this.storedDocs = applyMapDocChanges(this.storedDocs, changes);
			}
			this.base[N] = this.versions[entities.map]?.latest ?? null;
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

	/** publishing goes through the publish checklist (publish-tree.ts – every place its own line) */
	override async publish(): Promise<string[]> {
		throw new Error('the world map is published through the publish checklist');
	}
}
