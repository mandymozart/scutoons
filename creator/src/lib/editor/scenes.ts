// The creator's scene browser (#490, the first slice of draft 3 – research/SCENES.md §9, owner 2026-11-12 [scenes]: its
// own area "Scenes"): the scenes' rows, a scene's facts in words, and what a Publish answered. Nothing here decides
// whether a scene plays – that is the core's check behind Publish (RULES A2); these are the page's words for what the
// documents and the API say.
import { isSceneKey, sceneFacts, sceneState, type HolderFacts, type SceneFacts, type SceneListEntry, type SceneState } from '@scutoons/api';
import type { PublishRefusal } from '$lib/editor/publish-report.ts';

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

/** the scene's line of a Publish's answer (publish-tree.ts; null: not part of it – cancelled, or nothing to publish) */
export function publishOutcome(key: string, result: { published: string[]; refused: PublishRefusal[] }): PublishOutcome | null {
	if (result.published.includes(key)) return { kind: 'published' };
	const refusal = result.refused.find((r) => r.key === key);
	if (!refusal) return null;
	if (refusal.code === 'no_build_service' || refusal.code === 'build_unavailable') return { kind: 'unreachable' };
	if (refusal.code === 'invalid_scene' || refusal.code === 'invalid_content') return { kind: 'problems', problems: refusal.reason.split('; ') };
	return { kind: 'error', message: refusal.reason };
}

/** the scene's page in the creator (the MCP's link: tools/mcp services.ts creatorHref) */
export const sceneHref = (id: string) => `/scenes/${encodeURIComponent(id)}`;
