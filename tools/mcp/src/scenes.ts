// The deck scenes in the MCP (#490, owner 2026-11-12 [scenes]: "We need the mcp anyways"): list, read, the history and a
// new draft over `/api/scenes` (docs/API.md "Scenes" – `/api/entities` refuses `scene:` keys, so the generic content
// tools don't reach them). Format 2 (research/SCENES.md §9.3): scutoons://rules/scenes. The AI saves drafts and hands
// back the creator's scene page; publishing is the member's button there (RULES G2 – the API refuses an AI connection's
// publish anyway, 403 not_for_ai). No publish tool.
import type { McpServer } from '@modelcontextprotocol/server';
import * as z from 'zod';
import { ApiError, createApiClient, createScenesApi, isSceneKey, sceneFacts, sceneMatches, sceneState, type HolderFacts, type SceneFacts, type SceneListEntry, type WrappedContent } from '@scutoons/api';
import { ENTITY_ID } from '@scutoons/content';
import type { Config } from './config.ts';
import { apiMessage, applyOps } from './server.ts';
import { creatorHref, type AppLinks } from './services.ts';

type Result = { content: { type: 'text'; text: string }[]; isError?: boolean };
const text = (value: unknown): Result => ({ content: [{ type: 'text', text: typeof value === 'string' ? value : JSON.stringify(value, null, 1) }] });
const fail = (message: string): Result => ({ ...text(message), isError: true });

/** `scene:nave` or `nave` → `nave`; null: no scene id (the server's ID_PATTERN) */
export function sceneId(given: string): string | null {
	const id = given.startsWith('scene:') ? given.slice(6) : given;
	return ENTITY_ID.test(id) ? id : null;
}

/** where it plays, in words: `location:dom`, a placeholder `where (placeholder: location)`, or the space node's set */
export function whereWords(f: SceneFacts): string | null {
	const w = f.where;
	if (!w) return null;
	if (w.kind === 'location') return w.ref;
	if (w.kind === 'set') return `set ${w.set}`;
	return `${w.id} (placeholder: ${w.fill.join(' | ') || 'location'})`;
}

/** one holder in words: `catpope = character:catpope`, `left ← character (join)`, `crowd ← item × many (row, max 8)` */
export function holderWords(h: HolderFacts): string {
	const many = h.many ? ` × many (${h.many.arrange}${h.many.max != null ? `, max ${h.many.max}` : ''})` : '';
	const who = h.ref ? `= ${h.ref}` : `← ${(h.fill?.kind ?? []).join(' | ')}${h.fill?.ref.length ? ` of ${h.fill.ref.join(', ')}` : ''}${h.fill?.parent ? ` below ${h.fill.parent}` : ''}${many}`;
	return `${h.id} ${who}${h.spot ? ` at ${h.spot}` : ''}${h.enter === 'join' ? ' (join)' : ''}`;
}

/** a list row: what the agent picks a scene by (state: published · draft = a newer draft waits · unpublished = never) */
export function sceneRow(e: SceneListEntry, links: AppLinks) {
	const f = sceneFacts(e.key, e.content);
	return {
		id: f.id,
		key: e.key,
		name: f.name,
		toons: f.toons,
		where: whereWords(f),
		holders: f.holders.map(holderWords),
		state: sceneState(e),
		...(e.latest !== undefined ? { latest: e.latest, published: e.published ?? null } : {}),
		open: creatorHref(links, e.key)
	};
}

/** the list's filters (shared with the creator's browser): toon (`""` = no toon – the templates), a word, the state */
export function filterScenes(entries: SceneListEntry[], filter: { toon?: string; q?: string; state?: 'draft' | 'published' }): SceneListEntry[] {
	return entries.filter((e) => isSceneKey(e.key) && sceneMatches(sceneFacts(e.key, e.content), sceneState(e), { toon: filter.toon, query: filter.q, state: filter.state }));
}

/** a refused save the agent fixes in the document: the schema's problems (422 invalid_content); null = the usual error */
export function saveRefused(error: unknown) {
	if (!(error instanceof ApiError) || error.status !== 422) return null;
	return { saved: false, reason: error.message, problems: Array.isArray(error.body?.problems) ? error.body.problems : [] };
}

const versionMeta = (w: WrappedContent<unknown>) => ({
	id: w.version.id,
	status: w.version.status,
	created_at: w.version.created_at,
	author: w.version.author_email ?? w.version.author,
	message: w.version.message,
	published: w.published?.id ?? null
});

const idSchema = z.string().describe('the scene id (`nave`) or key (`scene:nave`)');

export function registerSceneTools(server: McpServer, config: Config, options: { write: boolean; links: AppLinks; fetch?: typeof fetch }): void {
	const scenes = createScenesApi(createApiClient({ baseUrl: config.api, getToken: () => config.token, fetch: options.fetch }));
	const read = { readOnlyHint: true, openWorldHint: false };
	const guard =
		<A>(fn: (args: A) => Promise<Result>) =>
		async (args: A): Promise<Result> => {
			try {
				return await fn(args);
			} catch (error) {
				return fail(apiMessage(error));
			}
		};
	const idOf = (given: string) => {
		const id = sceneId(given);
		if (!id) throw new Error(`"${given}" is no scene id (lowercase letters, digits, - and _)`);
		return id;
	};

	server.registerTool(
		'list_scenes',
		{
			description:
				'The deck scenes `scene:<id>` (platform-wide; format 2 – scutoons://rules/scenes): id, name, toons, where it plays, its holders in words, state (published / draft = a newer draft waits / unpublished = never published) and the creator page. `toon` "" = the scenes with no toon (the templates). Read one with read_scene.',
			inputSchema: z.object({
				toon: z.string().optional().describe('only scenes of this toon; "" = scenes with no toon'),
				q: z.string().optional().describe('a word in the id, name, location or holders'),
				state: z.enum(['draft', 'published']).optional().describe('draft = never published or a newer draft waits')
			}),
			annotations: read
		},
		guard(async ({ toon, q, state }) => {
			const { scenes: all } = await scenes.list();
			return text({ scenes: filterScenes(all, { toon, q, state }).map((e) => sceneRow(e, options.links)), of: all.length });
		})
	);

	server.registerTool(
		'read_scene',
		{
			description:
				'Reads one scene document (format 2: location, holder[] with ref or fill, take / arrange / max for a spawn zone, stage, node, link – scutoons://rules/scenes). `source` draft (default) = the newest version with `version.id` – pass it as `base` to save_scene_draft; published = what decks play; `stamp` = the document at its stamped `version` (what `scene:<id>@<n>` pins).',
			inputSchema: z.object({
				id: idSchema,
				source: z.enum(['draft', 'published']).optional().describe('default draft'),
				stamp: z.number().int().min(1).optional().describe('the document whose `version` field is this number (overrides source)')
			}),
			annotations: read
		},
		guard(async ({ id: given, source, stamp }) => {
			const id = idOf(given);
			const open = creatorHref(options.links, `scene:${id}`);
			if (stamp !== undefined) return text({ key: `scene:${id}`, stamp, content: (await scenes.atStamp(id, stamp)).scene, open });
			if (source === 'published') return text({ key: `scene:${id}`, source: 'published', content: (await scenes.published(id)).scene, open });
			const w = await scenes.draft(id);
			return text({ key: `scene:${id}`, version: versionMeta(w), content: w.content, open });
		})
	);

	server.registerTool(
		'list_scene_versions',
		{
			description: 'The version history of a scene (newest first): id, draft / published, author, message. Every save is its own version; the member restores one on the scene page.',
			inputSchema: z.object({ id: idSchema, limit: z.number().int().min(1).max(200).optional().describe('default 20') }),
			annotations: read
		},
		guard(async ({ id: given, limit }) => {
			const { versions } = await scenes.versions(idOf(given), limit ?? 20);
			return text(versions.map((v) => ({ id: v.id, status: v.status, created_at: v.created_at, author: v.author_email ?? v.author, message: v.message, size: v.size })));
		})
	);

	if (!options.write) return;

	server.registerTool(
		'save_scene_draft',
		{
			description:
				'Saves a NEW draft version of a scene (append-only; decks keep playing the published one). Give the whole `content` (format 2, `id` = the scene id; the server stamps `version`) or `ops` on the newest draft (base = its version id from read_scene; a newer save in between → 409). A new scene: content with base null. The server checks the shape (422 with the problems); whether a deck plays it is checked when the member publishes on the scene page (the `open` link) – you never publish.',
			inputSchema: z.object({
				id: idSchema,
				base: z.number().int().nullable().describe('the version id the change starts from (read_scene; null = a new scene)'),
				content: z.unknown().optional().describe('the whole scene document (instead of ops)'),
				ops: z
					.array(z.object({ op: z.enum(['set', 'insert', 'remove']), path: z.array(z.union([z.string(), z.number().int()])), value: z.unknown().optional() }))
					.optional()
					.describe('changes to the newest draft: set / insert (last path element = index or "-") / remove'),
				message: z.string().max(255).optional().describe('what changed (shown in the version history)')
			}),
			annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: false, openWorldHint: false }
		},
		guard(async ({ id: given, base, content, ops, message }) => {
			const id = idOf(given);
			if ((content === undefined) === !ops) return fail('give either ops or content');
			let doc = content;
			if (ops) {
				if (base === null) return fail('ops need a base version (read_scene)');
				const w = await scenes.draft(id);
				if (w.version.id !== base) return fail(`409 conflict: the newest draft is version ${w.version.id}, not ${base} – read_scene again, reapply your change and save with its version id as base.`);
				doc = applyOps(w.content, ops);
			}
			if (doc && typeof doc === 'object' && 'id' in doc && (doc as { id: unknown }).id !== id) return fail(`the document's id must be "${id}" (scene:${id})`);
			// a personal token: the API signs the save "(AI: <connection name>)"; a copied session (local dev) gets "(agent)"
			const note = message || 'AI edit';
			try {
				const r = await scenes.save(id, doc, base, config.token.startsWith('sct_') ? note : `${note} (agent)`);
				return text({ saved: !r.unchanged, unchanged: r.unchanged, version: r.version.id, status: r.version.status, open: creatorHref(options.links, `scene:${id}`), publish: 'the member publishes on the scene page (open)' });
			} catch (error) {
				const refused = saveRefused(error);
				if (refused) return { ...text(refused), isError: true };
				throw error;
			}
		})
	);
}
