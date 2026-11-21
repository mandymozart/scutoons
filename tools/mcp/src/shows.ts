// The live shows in the MCP (#643 follow-up, owner 2026-11-19: demos and tests in the sandbox toon, saved through the MCP):
// list, read and a new draft over `/api/shows` (docs/API.md "Live" – `/api/entities` refuses `show:` keys, so the generic
// content tools don't reach them; the server needs the member's role `live`). The AI saves drafts only; publishing is the
// member's (the editor's Show tab). No publish tool.
import type { McpServer } from '@modelcontextprotocol/server';
import * as z from 'zod';
import { createApiClient } from '@scutoons/api';
import type { Config } from './config.ts';
import { apiMessage, applyOps } from './server.ts';

type Result = { content: { type: 'text'; text: string }[]; isError?: boolean };
const text = (value: unknown): Result => ({ content: [{ type: 'text', text: typeof value === 'string' ? value : JSON.stringify(value, null, 1) }] });
const fail = (message: string): Result => ({ ...text(message), isError: true });

/** list_shows' rows: each API row { key, latest, published, content } (the newest draft's document, EntityStore::withContent) in
 *  short, only the shows that use `toon` when it is given (#708: a show names several toons, the first its home) */
export function showRows(all: Record<string, any>[], toon?: string) {
	return all
		.map((r): Record<string, any> => ({ ...((r.content ?? {}) as Record<string, any>), key: r.key, latest: r.latest, published: r.published }))
		.filter((s) => toon === undefined || (Array.isArray(s.toons) && s.toons.includes(toon)))
		.map((s) => ({ id: s.id ?? String(s.key).replace(/^show:/, ''), toons: Array.isArray(s.toons) ? s.toons : [], outlets: (s.outlets ?? []).map((o: { id: string }) => o.id), rows: (s.rows ?? []).length, draft: s.latest, published: s.published }));
}

/** a show id: lowercase letters, digits, - and _ (the server's ID_KINDS rule) */
export const SHOW_ID = /^[a-z0-9_-]{1,64}$/;
export function showId(given: string): string | null {
	const id = given.startsWith('show:') ? given.slice(5) : given;
	return SHOW_ID.test(id) ? id : null;
}

type Wrapped = { content: Record<string, unknown>; version: { id: number; status?: string } };

export function registerShowTools(server: McpServer, config: Config, options: { write: boolean; fetch?: typeof fetch }): void {
	const api = createApiClient({ baseUrl: config.api, getToken: () => config.token, fetch: options.fetch });
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
		const id = showId(given);
		if (!id) throw new Error(`"${given}" is no show id (lowercase letters, digits, - and _)`);
		return id;
	};
	const idSchema = z.string().min(1).describe('the show id (`demo-far-nodes`) or key (`show:demo-far-nodes`)');

	server.registerTool(
		'list_shows',
		{
			description: 'Every live show (the newest draft of each): id, toons (the first is its home), its outlets and rows in short. `toon` filters to shows that use that toon (e.g. "sandbox" for the demos and tests). Needs the role `live`.',
			inputSchema: z.object({ toon: z.string().optional() }),
			annotations: read
		},
		guard(async ({ toon }) => {
			const r = await api.get<{ shows?: unknown[] } | unknown[]>('/shows');
			const all = (Array.isArray(r) ? r : (r.shows ?? [])) as Record<string, any>[];
			const rows = showRows(all, toon);
			return text(rows);
		})
	);

	server.registerTool(
		'read_show',
		{
			description: 'Reads one live show: `source` draft (default) = the newest version with `version.id` (pass it as `base` to save_show_draft), published = what the conductor plays.',
			inputSchema: z.object({ id: idSchema, source: z.enum(['draft', 'published']).optional().describe('default draft') }),
			annotations: read
		},
		guard(async ({ id: given, source }) => {
			const id = idOf(given);
			const w = await api.get<Wrapped>(`/shows/${encodeURIComponent(id)}${source === 'published' ? '' : '?draft=1'}`);
			return text({ key: `show:${id}`, version: w.version, content: w.content });
		})
	);

	if (!options.write) return;

	server.registerTool(
		'save_show_draft',
		{
			description:
				'Saves a NEW draft version of a live show (append-only; the server stamps `version`). The whole `content` (`id` = the show id, `toons` – a list, the first the home toon –, `outlets`, `rows` …) or `ops` on the newest draft (base = its version id from read_show; a newer save in between → 409). A new show: content with base null. Demos and tests belong to the toon `sandbox`. You never publish – the member does, in the editor\'s Show tab.',
			inputSchema: z.object({
				id: idSchema,
				base: z.number().int().nullable().describe('the version id the change starts from (read_show; null = a new show)'),
				content: z.unknown().optional().describe('the whole show document (instead of ops)'),
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
				if (base === null) return fail('ops need a base version (read_show)');
				const w = await api.get<Wrapped>(`/shows/${encodeURIComponent(id)}?draft=1`);
				if (w.version.id !== base) return fail(`409 conflict: the newest draft is version ${w.version.id}, not ${base} – read_show again, reapply your change and save with its version id as base.`);
				doc = applyOps(w.content, ops);
			}
			if (doc && typeof doc === 'object' && 'id' in doc && (doc as { id: unknown }).id !== id) return fail(`the document's id must be "${id}" (show:${id})`);
			const note = message || 'AI edit';
			const r = await api.put<Wrapped & { unchanged?: boolean }>(`/shows/${encodeURIComponent(id)}`, { content: doc, base, message: config.token.startsWith('sct_') ? note : `${note} (agent)` });
			return text({ saved: !r.unchanged, version: r.version?.id, status: r.version?.status, publish: 'the member publishes in the editor (Show tab)' });
		})
	);
}
