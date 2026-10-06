// The season / toon level of the Flow (owner 2026-10-06, agents/DESIGN.md "Flow panel"): a toon's chapters, scenes,
// seasons and the map places that lead into it, read once (newest versions, the language-neutral documents; a not yet
// converted entity: one of its language files – the same jumps and variables) and shaped for @scutoons/content
// `toonFlow` / `deadThings`.
import { contentIndex, getDraft } from '$lib/api.ts';
import { deadThings, toonFacts, type DeadThings, type FlowChapter, type FlowPlace, type FlowSeason, type ToonFlowInput } from '@scutoons/content';

const KINDS = new Set(['chapter', 'scene', 'season', 'toon']);

/** a text (a string or a locale map) in `locale`, else the first one */
export function pickText(value: unknown, locale: string): string {
	if (typeof value === 'string') return value;
	if (!value || typeof value !== 'object') return '';
	const map = value as Record<string, unknown>;
	const pick = [locale, ...Object.keys(map)].map((l) => map[l]).find((t) => typeof t === 'string' && t.trim());
	return typeof pick === 'string' ? pick : '';
}

export interface ToonFlowData {
	input: ToonFlowInput;
	dead: DeadThings;
}

export async function loadToonFlow(toon: string, locale: string): Promise<ToonFlowData> {
	const index = (await contentIndex(toon)).entities.filter((e) => KINDS.has(e.kind));
	const base = (e: { key: string; locale: string | null }) => (e.locale ? e.key.slice(0, -(e.locale.length + 1)) : e.key);
	const neutral = new Set(index.filter((e) => !e.locale).map((e) => e.key));
	// one document per entity: the neutral one, else the first language file
	const chosen = new Map<string, string>();
	for (const e of [...index].sort((a, b) => a.key.localeCompare(b.key))) {
		const entity = base(e);
		if (e.locale && neutral.has(entity)) continue;
		if (!chosen.has(entity)) chosen.set(entity, e.key);
	}
	const [docs, map] = await Promise.all([
		Promise.all([...chosen].map(([entity, key]) => getDraft(key).then((r) => [entity, r.content] as const))),
		getDraft('map').then((r) => r.content as any).catch(() => null)
	]);
	const byEntity = new Map<string, any>(docs);
	const entry = byEntity.get(`toon:${toon}`);
	const source = typeof entry?.sourceLanguage === 'string' ? entry.sourceLanguage : locale;
	const seasons: FlowSeason[] = [];
	const statusOf = new Map<number, string>();
	for (const [entity, doc] of byEntity) {
		const [kind, , number] = entity.split(':');
		if (kind !== 'season' || number === undefined) continue;
		const items: any[] = Array.isArray(doc?.sequence) ? doc.sequence : [];
		const chapters = items.filter((i) => (i?.type ?? 'chapter') === 'chapter' && Number.isFinite(Number(i?.number))).map((i) => Number(i.number));
		for (const i of items) if (i?.status !== undefined) statusOf.set(Number(i.number), pickText(i.status, source));
		seasons.push({ number: Number(number), chapters });
	}
	seasons.sort((a, b) => a.number - b.number);
	const scene = (src: unknown) => {
		const match = typeof src === 'string' ? /^chapter(\d+)\/([a-z0-9_-]+)\.json$/.exec(src) : null;
		return match ? byEntity.get(`scene:${toon}:${match[1]}:${match[2]}`) : undefined;
	};
	const chapters: FlowChapter[] = [];
	for (const [entity, doc] of byEntity) {
		const [kind, , number] = entity.split(':');
		if (kind !== 'chapter' || number === undefined) continue;
		const groups = (Array.isArray(doc?.groups) ? doc.groups : []).map((g: any) => (g?.ref?.src ? (scene(g.ref.src) ?? g) : g));
		const n = Number(number);
		chapters.push({ number: n, groups, ...(statusOf.has(n) ? { status: statusOf.get(n) } : {}), ...(pickText(doc?.title, locale) ? { title: pickText(doc?.title, locale) } : {}) });
	}
	chapters.sort((a, b) => a.number - b.number);
	const places: FlowPlace[] = (Array.isArray(map?.locations) ? map.locations : [])
		.filter((l: any) => l?.chapter?.toon === toon && Number.isFinite(Number(l.chapter.number)))
		.map((l: any) => ({ id: String(l.id), name: pickText(l.name, locale) || String(l.id), chapter: Number(l.chapter.number), ...(l.hidden ? { hidden: true } : {}) }));
	const facts = toonFacts([...byEntity.values(), ...(map ? [map] : [])]);
	const input: ToonFlowInput = { toon, chapters, seasons, facts, places, text: (v) => pickText(v, locale) };
	return { input, dead: deadThings(chapters, facts) };
}
