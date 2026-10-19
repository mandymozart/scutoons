// A toon's documents shaped for `toonFlow` / `deadThings` (flow.ts): its chapters (each group's scene document in place
// of the reference), seasons (their chapter order and status), the map places that lead into it, the toon's facts
// (`toonFacts`, with what the other toons do from the server's flow index). Pure – the caller loads the documents (the
// creator's Flow panel, the MCP server's read_flow). Moved from creator/src/lib/editor/toon-flow.ts (agents/MCP.md).
import { deadThings, type DeadThings, type FlowChapter, type FlowPlace, type FlowSeason, type ToonFlowInput } from './flow.ts';
import { toonFacts } from './reach.ts';

/** a text (a string or a locale map) in `locale`, else the first one */
export function localeText(value: unknown, locale: string): string {
	if (typeof value === 'string') return value;
	if (!value || typeof value !== 'object') return '';
	const map = value as Record<string, unknown>;
	const pick = [locale, ...Object.keys(map)].map((l) => map[l]).find((t) => typeof t === 'string' && t.trim());
	return typeof pick === 'string' ? pick : '';
}

/** the entity keys a toon's flow reads: one document per entity – the language-neutral one, else its first language file */
export function toonFlowKeys(index: readonly { key: string; kind: string; locale: string | null }[]): Map<string, string> {
	const kinds = new Set(['chapter', 'season', 'toon']);
	const entries = index.filter((e) => kinds.has(e.kind));
	const base = (e: { key: string; locale: string | null }) => (e.locale ? e.key.slice(0, -(e.locale.length + 1)) : e.key);
	const neutral = new Set(entries.filter((e) => !e.locale).map((e) => e.key));
	const chosen = new Map<string, string>();
	for (const e of [...entries].sort((a, b) => a.key.localeCompare(b.key))) {
		const entity = base(e);
		if (e.locale && neutral.has(entity)) continue;
		if (!chosen.has(entity)) chosen.set(entity, e.key);
	}
	return chosen;
}

export interface ToonFlowSources {
	toon: string;
	/** the language names and choice texts are shown in */
	locale: string;
	/** entity → its document (`toonFlowKeys`) */
	docs: ReadonlyMap<string, any>;
	/** the world map document (`map`), null when there is none */
	map: any;
	/** what other toons jump into (chapters + labels) and the variables they set / read (`incomingJumps` / `variablesElsewhere`) */
	incoming: { chapters: number[]; labels: string[]; elsewhere: { sets: string[]; reads: string[] } };
}

export function toonFlowInput(sources: ToonFlowSources): { input: ToonFlowInput; dead: DeadThings } {
	const { toon, locale, docs: byEntity, map, incoming } = sources;
	const entry = byEntity.get(`toon:${toon}`);
	const source = typeof entry?.sourceLanguage === 'string' ? entry.sourceLanguage : locale;
	const seasons: FlowSeason[] = [];
	const statusOf = new Map<number, string>();
	for (const [entity, doc] of byEntity) {
		const [kind, , number] = entity.split(':');
		if (kind !== 'season' || number === undefined) continue;
		const items: any[] = Array.isArray(doc?.sequence) ? doc.sequence : [];
		const chapters = items.filter((i) => (i?.type ?? 'chapter') === 'chapter' && Number.isFinite(Number(i?.number))).map((i) => Number(i.number));
		for (const i of items) if (i?.status !== undefined) statusOf.set(Number(i.number), localeText(i.status, source));
		seasons.push({ number: Number(number), chapters });
	}
	seasons.sort((a, b) => a.number - b.number);
	const chapters: FlowChapter[] = [];
	for (const [entity, doc] of byEntity) {
		const [kind, , number] = entity.split(':');
		if (kind !== 'chapter' || number === undefined) continue;
		const groups = Array.isArray(doc?.groups) ? doc.groups : [];
		const n = Number(number);
		chapters.push({
			number: n,
			groups,
			...(statusOf.has(n) ? { status: statusOf.get(n) } : {}),
			...(localeText(doc?.title, locale) ? { title: localeText(doc?.title, locale) } : {}),
			// the box's picture (images in the Flow)
			...(typeof doc?.thumbnail === 'string' && doc.thumbnail ? { thumbnail: doc.thumbnail } : {})
		});
	}
	chapters.sort((a, b) => a.number - b.number);
	const places: FlowPlace[] = (Array.isArray(map?.locations) ? map.locations : [])
		.filter((l: any) => l?.chapter?.toon === toon && Number.isFinite(Number(l.chapter.number)))
		.map((l: any) => ({ id: String(l.id), name: localeText(l.name, locale) || String(l.id), chapter: Number(l.chapter.number), ...(l.hidden ? { hidden: true } : {}) }));
	const facts = toonFacts([...byEntity.values(), ...(map ? [map] : [])], { toon, incoming: incoming.labels, elsewhere: incoming.elsewhere });
	const input: ToonFlowInput = { toon, chapters, seasons, facts, places, incoming: incoming.chapters, text: (v) => localeText(v, locale) };
	return { input, dead: deadThings(chapters, facts) };
}
