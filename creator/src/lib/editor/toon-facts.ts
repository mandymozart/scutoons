// The toon-wide facts for the reachability check (owner 2026-10-06, @scutoons/content reach.ts): which labels something
// jumps to, which variables something sets – read from every chapter / scene of the toon (newest versions, language-
// neutral documents) once per editor. The open chapter's own documents are passed live by the editor (they change).
import { api, contentIndex, getDraft } from '$lib/api.ts';
import { incomingJumps, variablesElsewhere, type FlowIndex } from '@scutoons/content';

/** the kinds whose documents can jump somewhere or set variables */
const KINDS = new Set(['chapter', 'scene', 'map']);

/**
 * the toon's language-neutral chapter / scene / map documents (newest versions), except `skip` (keys the editor has
 * open). All or nothing: a document that doesn't load would make a variable look "set nowhere" – it throws then.
 */
export async function loadToonDocs(toon: string, skip: ReadonlySet<string> = new Set()): Promise<unknown[]> {
	const index = (await contentIndex(toon)).entities.filter((e) => KINDS.has(e.kind));
	const base = (e: { key: string; locale: string | null }) => (e.locale ? e.key.slice(0, -(e.locale.length + 1)) : e.key);
	const neutral = new Set(index.filter((e) => !e.locale).map((e) => e.key));
	// the neutral document; a not yet converted one: its language files (the same jumps and variables)
	const keys = index.filter((e) => !skip.has(base(e)) && (!e.locale || !neutral.has(base(e)))).map((e) => e.key);
	return Promise.all(keys.map((key) => getDraft(key).then((r) => r.content)));
}

/**
 * What the other toons tell about `toon` – from the server's flow index (`GET /api/creator/flow-index`, cached there):
 * their jumps into it (chapters + labels, owner 2026-10-06 "at label") and the story variables they set / read (one save
 * game for every toon: `toonFacts(…, { elsewhere })`). Nothing when it doesn't load (a label may then look unreached, a
 * variable set only in another toon unknown).
 */
/**
 * Which documents unlock which map place (unlockLocation events in every toon – the server's flow index): the map
 * builder warns about a hidden place nothing unlocks and before a place with events is deleted. null when it doesn't load.
 */
export async function loadPlaceUnlocks(): Promise<Record<string, string[]> | null> {
	try {
		const index = await api.get<FlowIndex>('/creator/flow-index');
		return index.placeUnlocks ?? {};
	} catch {
		return null;
	}
}

export async function loadIncoming(toon: string): Promise<{ chapters: number[]; labels: string[]; elsewhere: { sets: string[]; reads: string[] } }> {
	try {
		const index = await api.get<FlowIndex>('/creator/flow-index');
		return { ...incomingJumps(index, toon), elsewhere: variablesElsewhere(index, toon) };
	} catch {
		return { chapters: [], labels: [], elsewhere: { sets: [], reads: [] } };
	}
}
