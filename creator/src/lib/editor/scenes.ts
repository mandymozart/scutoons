// The creator's scene browser (#490, the first slice of draft 3 – research/SCENES.md §9, owner 2026-11-12 [scenes]: its
// own area "Scenes", a scene with no toon listed under "no toon"): the list's rows and filters, a scene's facts in
// words, and what a Publish answered. Nothing here decides whether a scene plays – that is the core's check behind
// Publish (RULES A2); these are the page's words for what the documents and the API say.
import { ApiError, isSceneKey, sceneFacts, sceneMatches, sceneState, type HolderFacts, type SceneFacts, type SceneListEntry, type SceneState } from '@scutoons/api';

/** the toon filter's value for the scenes with no toon (the templates) */
export const NO_TOON = '__none';

export interface SceneRow {
	id: string;
	key: string;
	facts: SceneFacts;
	state: SceneState;
	/** the newest version's id (the base of a draft, what Publish publishes) */
	latest: number | null;
}

/** the list: every scene, by name (else id) – deck scenes only (an old chapter group key of a stale local seed is skipped) */
export function sceneRows(entries: SceneListEntry[]): SceneRow[] {
	return entries
		.filter((e) => isSceneKey(e.key))
		.map((e) => {
			const facts = sceneFacts(e.key, e.content);
			return { id: facts.id, key: e.key, facts, state: sceneState(e), latest: e.latest ?? null };
		})
		.sort((a, b) => sceneTitle(a.facts).localeCompare(sceneTitle(b.facts)));
}

/** the name for people: the stage's name, else the id */
export const sceneTitle = (f: SceneFacts) => f.name?.trim() || f.id;

export interface SceneFilter {
	query: string;
	/** '' = every toon, NO_TOON = the scenes with no toon, else a toon id */
	toon: string;
	state: 'all' | 'draft' | 'published';
}

export function filterRows(rows: SceneRow[], filter: SceneFilter): SceneRow[] {
	const toon = filter.toon === NO_TOON ? '' : filter.toon || undefined;
	const state = filter.state === 'all' ? undefined : filter.state;
	return rows.filter((r) => sceneMatches(r.facts, r.state, { toon, query: filter.query, state }));
}

/** the toons the scenes name (the filter's options), sorted; `noToon` = some scene has none */
export function sceneToons(rows: SceneRow[]): { toons: string[]; noToon: boolean } {
	return { toons: [...new Set(rows.flatMap((r) => r.facts.toons))].sort(), noToon: rows.some((r) => !r.facts.toons.length) };
}

/** the row's second line: where · spot · who (`Cathedral · altar · Cat Pope`) */
export function rowHint(f: SceneFacts, name: (ref: string) => string = refName): string {
	const where = f.where?.kind === 'location' ? name(f.where.ref) : f.where?.kind === 'set' ? f.where.set : f.where ? `‹${f.where.id}›` : null;
	const who = f.holders.map((h) => (h.ref ? name(h.ref) : `‹${h.id}›`));
	return [where, f.spot, ...who].filter(Boolean).join(' · ');
}

/** a reference's own id for people while no address book names it: `character:catpope` → catpope, `location:dom` → dom */
export const refName = (ref: string) => ref.replace(/^[a-z-]+:/, '');

/** a placeholder's filter in words: `character | persona of character:catpope below …` */
export function fillWords(fill: NonNullable<HolderFacts['fill']>, words: { of: string; below: string }): string {
	return [fill.kind.join(' | '), fill.ref.length ? `${words.of} ${fill.ref.join(', ')}` : '', fill.parent ? `${words.below} ${fill.parent}` : ''].filter(Boolean).join(' ');
}

/** a label map in the page's language (else the first one) */
export function labelIn(label: Record<string, string> | null | undefined, locale: string): string | null {
	if (!label) return null;
	return label[locale] ?? Object.values(label)[0] ?? null;
}

/** what a Publish answered, as the page shows it */
export type PublishOutcome =
	| { kind: 'published' }
	/** 422 invalid_scene / invalid_content: the core's (or the schema's) problems – the list the member fixes */
	| { kind: 'problems'; problems: string[] }
	/** 503: the build service that checks scenes is not set up or does not answer – try again */
	| { kind: 'unreachable' }
	| { kind: 'error'; message: string };

export function publishOutcome(error: unknown): PublishOutcome {
	if (error === null || error === undefined) return { kind: 'published' };
	if (error instanceof ApiError) {
		if (error.status === 422) {
			const problems = Array.isArray(error.body?.problems) ? error.body.problems.map(String) : [];
			return { kind: 'problems', problems: problems.length ? problems : [error.message] };
		}
		if (error.status === 503) return { kind: 'unreachable' };
		return { kind: 'error', message: error.message };
	}
	return { kind: 'error', message: error instanceof Error ? error.message : String(error) };
}

/**
 * may the batch Publish (publish-tree.ts) offer this file? Not a deck scene (#493): it is published on its own page,
 * where the core's check answers – `/api/entities/publish` refuses it and with it the whole batch
 */
export const inPublishBatch = (key: string) => !isSceneKey(key);

/** the scene's page in the creator (the MCP's link: tools/mcp services.ts creatorHref) */
export const sceneHref = (id: string) => `/scenes/${encodeURIComponent(id)}`;
