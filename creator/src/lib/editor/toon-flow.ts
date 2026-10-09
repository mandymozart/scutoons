// The season / toon level of the Flow (owner 2026-10-06, agents/DESIGN.md "Flow panel"): a toon's chapters, scenes,
// seasons and the map places that lead into it, read once (newest versions, the language-neutral documents; a not yet
// converted entity: one of its language files – the same jumps and variables) and shaped by @scutoons/content
// `toonFlowInput` for `toonFlow` / `deadThings` (the MCP server's read_flow shapes them the same way).
import { contentIndex, getDraft } from '$lib/api.ts';
import { loadIncoming } from '$lib/editor/toon-facts.ts';
import { localeText, toonFlowInput, toonFlowKeys, type DeadThings, type ToonFlowInput } from '@scutoons/content';

/** a text (a string or a locale map) in `locale`, else the first one */
export const pickText = localeText;

export interface ToonFlowData {
	input: ToonFlowInput;
	dead: DeadThings;
}

export async function loadToonFlow(toon: string, locale: string): Promise<ToonFlowData> {
	const chosen = toonFlowKeys((await contentIndex(toon)).entities);
	const [docs, map, incoming] = await Promise.all([
		Promise.all([...chosen].map(([entity, key]) => getDraft(key).then((r) => [entity, r.content] as const))),
		getDraft('map').then((r) => r.content as any).catch(() => null),
		// what other toons jump into (their chapters + labels are entries here)
		loadIncoming(toon)
	]);
	return toonFlowInput({ toon, locale, docs: new Map<string, any>(docs), map, incoming });
}
