// Tools for questions about the story as a whole (agents/MCP.md "foresight", owner 2026-10-09: "we have tools we will
// need when asking complex questions"): the flow at every level (SCUTOONS → toon → chapter → group), the story
// variables of the one save game, what nothing reaches, where readers can get stuck, where an item is placed, where a
// file is used, the review queue, what changed between two versions. Read only. The same functions the creator's
// Flow panel and checks use (@scutoons/content flow.ts, reach.ts, stuck.ts, toon-flow-input.ts) – so the AI sees what
// the creator shows.
import type { McpServer } from '@modelcontextprotocol/server';
import * as z from 'zod';
import { createApiClient, createAssetsApi, createContentApi } from '@scutoons/api';
import {
	chapterExits,
	chapterFlow,
	groupFlow,
	incomingJumps,
	stuckPoints,
	toonFlow,
	toonFlowInput,
	toonFlowKeys,
	universeFlow,
	variablesElsewhere,
	type FlowEdge,
	type FlowIndex,
	type FlowNode,
	MAP_COLLECTION,
	collectionDocsFromEntities,
	composeWorldMap
} from '@scutoons/content';
import type { Config } from './config.ts';
import { apiMessage } from './server.ts';

type Result = { content: { type: 'text'; text: string }[]; isError?: boolean };
// compact JSON: flows and diffs are long
const text = (value: unknown): Result => ({ content: [{ type: 'text', text: typeof value === 'string' ? value : JSON.stringify(value) }] });
const fail = (message: string): Result => ({ ...text(message), isError: true });
const guard =
	<A>(fn: (args: A) => Promise<Result>) =>
	async (args: A): Promise<Result> => {
		try {
			return await fn(args);
		} catch (error) {
			return fail(apiMessage(error));
		}
	};
const read = { readOnlyHint: true, openWorldHint: false };

/** a node as the AI reads it: what it is, its state, where it lives (path → read_entity `path`) */
const node = (n: FlowNode) => {
	const { id, kind, status, name, group, count, slice, trigger, repeat, target, next, path, deadSlices, script } = n;
	return Object.fromEntries(Object.entries({ id, kind, status, name, group, count, slice, trigger, repeat, target, next, path, deadSlices, script }).filter(([, v]) => v !== undefined));
};
const edge = (e: FlowEdge) => Object.fromEntries(Object.entries(e).filter(([k, v]) => v !== undefined && k !== 'id'));

/** where two JSON values differ: added / removed / changed paths (values cut to 200 characters) */
export function diffJson(a: unknown, b: unknown, max = 300): { path: (string | number)[]; change: 'added' | 'removed' | 'changed'; from?: unknown; to?: unknown }[] {
	const out: ReturnType<typeof diffJson> = [];
	const cut = (v: unknown) => {
		const s = JSON.stringify(v);
		return s !== undefined && s.length > 200 ? `${s.slice(0, 200)}…` : v;
	};
	const walk = (x: any, y: any, path: (string | number)[]) => {
		if (out.length >= max) return;
		if (JSON.stringify(x) === JSON.stringify(y)) return;
		const obj = (v: any) => v !== null && typeof v === 'object';
		if (obj(x) && obj(y) && Array.isArray(x) === Array.isArray(y)) {
			const keys = Array.isArray(x) ? [...Array(Math.max(x.length, y.length)).keys()] : [...new Set([...Object.keys(x), ...Object.keys(y)])];
			for (const k of keys) {
				const has = (v: any) => (Array.isArray(v) ? (k as number) < v.length : Object.hasOwn(v, k));
				if (!has(x)) out.push({ path: [...path, k], change: 'added', to: cut(y[k]) });
				else if (!has(y)) out.push({ path: [...path, k], change: 'removed', from: cut(x[k]) });
				else walk(x[k], y[k], [...path, k]);
				if (out.length >= max) return;
			}
			return;
		}
		out.push({ path, change: 'changed', from: cut(x), to: cut(y) });
	};
	walk(a, b, []);
	return out;
}

export function registerStoryTools(server: McpServer, config: Config): void {
	const client = createApiClient({ baseUrl: config.api, getToken: () => config.token });
	const content = createContentApi(client);
	const assets = createAssetsApi(client);

	const flowIndex = () => client.get<FlowIndex>('/creator/flow-index');

	/** a toon's flow sources (newest versions), shaped like the creator's Flow panel */
	async function toonSources(toon: string, locale: string | undefined) {
		const index = (await content.contentIndex(toon)).entities;
		if (!index.length) throw new Error(`no documents for toon "${toon}" (list_toons)`);
		const chosen = toonFlowKeys(index);
		const [docs, map, fi] = await Promise.all([
			Promise.all([...chosen].map(([entity, key]) => content.getDraft(key).then((r) => [entity, r.content] as const))),
			// the world map composed from its documents (map, place:<id>, route:<id> – agents/MAP-DOCUMENTS.md)
			content.withContent(['map', 'place', 'route'], 'draft').then((r) => { const d = collectionDocsFromEntities(MAP_COLLECTION, r.entities); return d.settings ? composeWorldMap(d) : null; }).catch(() => null),
			flowIndex().catch(() => null)
		]);
		const byEntity = new Map<string, any>(docs);
		const entry = byEntity.get(`toon:${toon}`);
		const lang = locale ?? (typeof entry?.sourceLanguage === 'string' ? entry.sourceLanguage : 'de');
		const incoming = { ...incomingJumps(fi, toon), elsewhere: variablesElsewhere(fi, toon) };
		return { ...toonFlowInput({ toon, locale: lang, docs: byEntity, map, incoming }), locale: lang, crossToon: !!fi };
	}

	server.registerTool(
		'read_flow',
		{
			description:
				'The story flow as the creator\'s Flow panel shows it. No toon: SCUTOONS – one box per toon, the jumps between toons and to products / XR. A toon: its chapters (by season) and how they lead into each other, which can be reached (`status`), map places that lead in. A chapter: its groups, labels, choices (with their texts and tests), jumps, triggers and exits, plus where readers can get stuck. A group (index or label): that group only. Node `path` → read_entity `path` on the chapter / scene.',
			inputSchema: z.object({
				toon: z.string().optional().describe('left out = all of SCUTOONS'),
				season: z.number().int().optional().describe('toon level: only this season'),
				chapter: z.number().int().optional(),
				group: z.union([z.number().int(), z.string()]).optional().describe('with chapter: a group index or label'),
				locale: z.string().optional().describe('the language texts are shown in (default the toon\'s source language)')
			}),
			annotations: read
		},
		guard(async ({ toon, season, chapter, group, locale }) => {
			if (!toon) {
				const fi = await flowIndex();
				const flow = universeFlow(fi);
				return text({ level: 'scutoons', toons: fi.toons, nodes: flow.nodes.map(node), edges: flow.edges.map(edge) });
			}
			const src = await toonSources(toon, locale);
			if (chapter === undefined) {
				const flow = toonFlow(src.input, season !== undefined ? { season } : {});
				return text({
					level: 'toon',
					toon,
					locale: src.locale,
					seasons: src.input.seasons,
					cross_toon: src.crossToon ? 'included' : 'the flow index did not load – jumps from other toons are missing',
					nodes: flow.nodes.map(node),
					edges: flow.edges.map(edge)
				});
			}
			const ch = src.input.chapters.find((c) => c.number === chapter);
			if (!ch) return fail(`${toon} has no chapter ${chapter} (read_flow without chapter lists them)`);
			const groups = ch.groups as any[];
			const options = { chapter, toon, facts: src.input.facts, text: src.input.text };
			let gi: number | undefined;
			if (group !== undefined) {
				gi = typeof group === 'number' ? group : groups.findIndex((g) => g?.label === group);
				if (gi < 0 || gi >= groups.length) return fail(`chapter ${chapter} has no group ${JSON.stringify(group)} (${groups.length} groups; labels: ${groups.map((g) => g?.label).filter(Boolean).join(', ') || 'none'})`);
			}
			const flow = gi === undefined ? chapterFlow(groups, options) : groupFlow(groups, gi, options);
			return text({
				level: gi === undefined ? 'chapter' : 'group',
				toon,
				chapter,
				...(gi !== undefined ? { group: gi } : {}),
				locale: src.locale,
				nodes: flow.nodes.map(node),
				edges: flow.edges.map(edge),
				exits: gi === undefined ? chapterExits(flow) : undefined,
				stuck: stuckPoints(gi === undefined ? groups : [groups[gi]]).map((s) => (gi === undefined ? s : { ...s, path: ['groups', gi, ...s.path.slice(2)] }))
			});
		})
	);

	server.registerTool(
		'story_variables',
		{
			description:
				'The story variables of the one save game (every toon shares it): which toons set and read each one. Storage keys: `catpope/coins` = the toon\'s own `coins`, `coins` without a toon = `global.coins`. A name filter (part of the key) and / or a toon. For where inside a toon: search_content.',
			inputSchema: z.object({ name: z.string().optional(), toon: z.string().optional() }),
			annotations: read
		},
		guard(async ({ name, toon }) => {
			const fi = await flowIndex();
			const rows = Object.entries(fi.variables ?? {})
				.filter(([key, v]) => (!name || key.includes(name)) && (!toon || v.set.includes(toon) || v.read.includes(toon)))
				.map(([key, v]) => ({ key, set_in: v.set, read_in: v.read, ...(v.set.length === 0 ? { problem: 'read but set nowhere – its tests are never true' } : v.read.length === 0 ? { problem: 'set but read nowhere' } : {}) }));
			return text({ count: rows.length, variables: rows });
		})
	);

	server.registerTool(
		'dead_things',
		{
			description:
				'What nothing reaches or uses, judged across every toon (the creator\'s Flow lists the same): variables set but never read, variables read but never set, script points nothing diverts to, group labels nothing jumps to. One toon or all.',
			inputSchema: z.object({ toon: z.string().optional() }),
			annotations: read
		},
		guard(async ({ toon }) => {
			const fi = await flowIndex();
			const dead = fi.dead ?? {};
			if (toon) return dead[toon] ? text({ toon, ...dead[toon] }) : fail(`no flow data for "${toon}" (list_toons)`);
			return text(dead);
		})
	);

	server.registerTool(
		'item_placements',
		{
			description: 'Where a catalog item (by id, e.g. "camera") is placed in the content – chapter / scene / map, draft or published – with its limits (unique, respawn). For "where can readers get X".',
			inputSchema: z.object({ item: z.string().regex(/^[A-Za-z0-9_.-]{1,100}$/) }),
			annotations: read
		},
		guard(async ({ item }) => text(await client.get(`/catalog/items/${encodeURIComponent(item)}/placements`)))
	);

	server.registerTool(
		'asset_usages',
		{
			description:
				'One media file by its CDN path (e.g. "catpope/items/camera.webp"): the asset (id, versions, renditions) and every place that uses it – content documents (draft / published) and catalog entries (`item:camera` …).',
			inputSchema: z.object({ path: z.string().min(1) }),
			annotations: read
		},
		guard(async ({ path }) => {
			const found = await assets.lookup(path);
			if (!found.asset) return fail(`No asset has the file ${path} (search_assets finds files).`);
			const a = found.asset;
			return text({
				asset: a.asset,
				current: a.versions.at(-1) ?? null,
				versions: a.versions.length,
				usages: a.usages.map((u) => ({ ref: u.ref, source: u.source, field: u.field, state: u.state, path: u.path }))
			});
		})
	);

	server.registerTool(
		'missing_assets',
		{
			description: 'Media paths the content uses that have no file in the asset manager (broken pictures, sounds, videos) – where each one is used.',
			inputSchema: z.object({ toon: z.string().optional().describe('only paths of this toon') }),
			annotations: read
		},
		guard(async ({ toon }) => {
			const r = await assets.missing();
			const items = toon ? r.items.filter((i: any) => JSON.stringify(i).includes(toon)) : r.items;
			return text({ count: items.length, items });
		})
	);

	server.registerTool(
		'list_reviews',
		{
			description: 'The review queue: what waits for this member to review (to_review), what they handed in (mine), and the toons where they need a review before publishing.',
			inputSchema: z.object({}),
			annotations: read
		},
		guard(async () => text(await content.reviews()))
	);

	server.registerTool(
		'compare_versions',
		{
			description:
				'What changed between two versions of a stored document (list_versions gives the ids): every added / removed / changed path with the old and new value. `to` left out = the newest version. For "what did I (or someone) change", or before restoring.',
			inputSchema: z.object({ key: z.string(), from: z.number().int(), to: z.number().int().optional(), path: z.array(z.union([z.string(), z.number()])).optional().describe('only below this path') }),
			annotations: read
		},
		guard(async ({ key, from, to, path }) => {
			const [a, b] = await Promise.all([content.getVersion<any>(key, from), to === undefined ? content.getDraft<any>(key) : content.getVersion<any>(key, to)]);
			const at = (doc: any) => (path ?? []).reduce((v: any, k) => (v == null ? v : v[k]), doc);
			const changes = diffJson(at(a.content), at(b.content));
			return text({ key, from: a.version.id, to: b.version.id, changes: changes.length, ...(changes.length >= 300 ? { note: 'the first 300 changes – narrow it with path' } : {}), diff: changes });
		})
	);
}
