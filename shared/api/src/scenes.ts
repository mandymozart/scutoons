// The deck scenes `scene:<id>` (server `/api/scenes`, docs/API.md "Scenes"; research/SCENES.md §9.3 format 2): read by
// everyone when published, drafts and the history by the content editors; a save is a new draft, Publish asks the
// core's check first (422 `invalid_scene { problems }`, 503 without the build service) and never takes an AI
// connection's token (403 `not_for_ai`, RULES G2). `/api/entities` refuses `scene:` keys – this is their one route.
import type { ApiClient } from './client.ts';
import type { ContentVersion, WrappedContent } from './content.ts';
import type { SceneDocument } from './scene.ts';

/**
 * One scene of `GET /api/scenes`. A reader gets only `key` + `content` (the published document); a content editor every
 * scene with its newest document and its versions (as `GET /api/entities?kinds=`).
 */
export interface SceneListEntry {
	key: string;
	content: SceneDocument | null;
	kind?: string;
	/** the id of its first version (the creation order) */
	first?: number;
	/** the newest version's id */
	latest?: number;
	/** the published version's id, null: never published */
	published?: number | null;
	/** the newest version is the same document as the published one */
	same_as_published?: boolean;
	/** the version `content` is */
	version?: number | null;
}

/**
 * a deck scene's key `scene:<id>` (server EntityKey ID_PATTERN) – not an old chapter group `scene:<toon>:<n>:<name>:<l>`
 * (archived 2026-10-19; a local DB loaded from an older seed still has them, and the index lists them under `scene`)
 */
export const isSceneKey = (key: string) => /^scene:[a-z0-9][a-z0-9_-]{0,63}$/.test(key);

/** published · draft (a newer draft waits, or never published) – what the creator's badges and the MCP's list say */
export type SceneState = 'published' | 'draft' | 'unpublished';

/** a reader's entry (no versions) is published; a restore of the published document is nothing new (owner D6) */
export function sceneState(e: SceneListEntry): SceneState {
	if (e.latest === undefined) return 'published';
	if (e.published == null) return 'unpublished';
	return e.latest !== e.published && !e.same_as_published ? 'draft' : 'published';
}

/** where a scene plays: a location, a location placeholder a use fills, or (no location) the `space` node's own set */
export type SceneWhere =
	| { kind: 'location'; ref: string }
	| { kind: 'placeholder'; id: string; fill: string[]; label: Record<string, string> | null; preview: string | null }
	| { kind: 'set'; set: string };

/** one holder as read (SCENES.md §9.3): a filler, or a placeholder's filter; a spawn zone takes many */
export interface HolderFacts {
	id: string;
	ref: string | null;
	fill: { kind: string[]; ref: string[]; parent: string | null } | null;
	many: { arrange: string; spacing: number | null; max: number | null } | null;
	spot: string | null;
	enter: 'start' | 'join';
	rig: string | null;
	label: Record<string, string> | null;
}

/** the facts a list row and a scene's page show – read from the document, nothing decided (whether it plays is the core's) */
export interface SceneFacts {
	id: string;
	name: string | null;
	toons: string[];
	where: SceneWhere | null;
	/** the `space` node's spot: where the scene stands in its set */
	spot: string | null;
	holders: HolderFacts[];
	/** the number the server stamped (what `scene:<id>@<n>` pins) */
	stamp: number | null;
}

export function sceneFacts(key: string, doc: SceneDocument | null): SceneFacts {
	const space = (doc?.node ?? []).find((n) => n.kind === 'space');
	const loc = doc?.location;
	const where: SceneWhere | null =
		typeof loc === 'string'
			? { kind: 'location', ref: loc }
			: loc && typeof loc === 'object'
				? { kind: 'placeholder', id: loc.id, fill: [...(loc.fill?.kind ?? [])], label: loc.label ?? null, preview: loc.preview ?? null }
				: typeof space?.set === 'string'
					? { kind: 'set', set: space.set }
					: null;
	return {
		id: key.replace(/^scene:/, ''),
		name: doc?.stage?.name ?? null,
		toons: doc?.toons ?? [],
		where,
		spot: typeof space?.spot === 'string' ? space.spot : null,
		holders: (doc?.holder ?? []).map((h) => ({
			id: h.id,
			ref: h.ref ?? null,
			fill: h.fill ? { kind: [...h.fill.kind], ref: h.fill.ref ?? [], parent: h.fill.parent ?? null } : null,
			many: h.take === 'many' ? { arrange: h.arrange?.kind ?? 'row', spacing: h.arrange?.spacing ?? null, max: h.max ?? null } : null,
			spot: h.spot ?? null,
			enter: h.enter ?? 'start',
			rig: h.rig?.ref ?? null,
			label: h.label ?? null
		})),
		stamp: doc?.version ?? null
	};
}

/**
 * The scene browser's filters (draft 3, #490): a toon (`''` = the scenes with no toon – the templates; undefined = all),
 * one search over id, name, location / set, spot and the holders' fillers and filters, the state
 */
export function sceneMatches(f: SceneFacts, state: SceneState, filter: { toon?: string; query?: string; state?: 'draft' | 'published' }): boolean {
	if (filter.toon !== undefined && (filter.toon === '' ? f.toons.length > 0 : !f.toons.includes(filter.toon))) return false;
	if (filter.state && (filter.state === 'published') !== (state === 'published')) return false;
	const q = filter.query?.trim().toLowerCase();
	if (!q) return true;
	const where = f.where?.kind === 'location' ? f.where.ref : f.where?.kind === 'set' ? f.where.set : f.where ? [f.where.id, ...f.where.fill, f.where.preview ?? ''].join(' ') : '';
	const holders = f.holders.flatMap((h) => [h.id, h.ref ?? '', ...(h.fill?.kind ?? []), ...(h.fill?.ref ?? []), h.fill?.parent ?? '', h.rig ?? '', ...Object.values(h.label ?? {})]);
	return [f.id, f.name ?? '', where, f.spot ?? '', ...holders].some((s) => s.toLowerCase().includes(q));
}

export type ScenesApi = ReturnType<typeof createScenesApi>;

export function createScenesApi(client: ApiClient) {
	const at = (id: string) => `/scenes/${encodeURIComponent(id)}`;
	return {
		/** every scene (a content editor: drafts too, with their versions) */
		list: () => client.get<{ status: string; scenes: SceneListEntry[] }>('/scenes'),
		/** the newest version (draft or published) with its meta – content editors; ApiError 404 when none is stored */
		draft: (id: string) => client.get<WrappedContent<SceneDocument>>(`${at(id)}?draft=1`),
		/** the published document; ApiError 404 when none is published */
		published: (id: string) => client.get<{ status: string; scene: SceneDocument }>(at(id)),
		/** the document at its stamped `version` (what `scene:<id>@<n>` pins; a reader: only the published one) */
		atStamp: (id: string, stamp: number) => client.get<{ status: string; scene: SceneDocument }>(`${at(id)}?v=${stamp}`),
		/** the history, newest first (content editors) */
		versions: (id: string, limit?: number) => client.get<{ status: string; versions: ContentVersion[] }>(`${at(id)}/versions${limit ? `?limit=${limit}` : ''}`),
		/** a new draft; `base` = the version id the change starts from (409 when a newer one was saved); 422 `invalid_content { problems }` */
		save: (id: string, content: unknown, base?: number | null, message?: string) =>
			client.put<{ status: string; unchanged: boolean; version: ContentVersion }>(at(id), { content, ...(base !== undefined ? { base } : {}), message: message || undefined }),
		/** publishes the newest (or the given) version after the core's check – never with an AI connection's token */
		publish: (id: string, version?: number) => client.post<{ status: string; version: ContentVersion }>(`${at(id)}/publish`, version !== undefined ? { version } : {}),
		/** an older version (its id) as the newest draft */
		restore: (id: string, version: number, message?: string) => client.post<{ status: string; version: ContentVersion }>(`${at(id)}/restore`, { id: version, message: message || undefined })
	};
}
