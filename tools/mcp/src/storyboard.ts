// The storyboard through the MCP (#122, agents/research/STUDIO-IN-CREATOR.md §2.3; owner 2026-10-20: "the storyboard …
// can still be nicely remote controlled through the mcp"): the chapter document `storyboard:<toon>:<chapter>` read in
// short (an outline with the JSON paths – read_entity / save_draft do the rest, saves are drafts as always), checked
// (content's validateChapter + the rhythm), a shot's Luma prompt, and the generations – the records of what was made
// for which shot (`storyboard_generations`, server/docs/API.md "Storyboard"): listed, marked, approved, and recorded
// when a member's own tools made one. Paid provider jobs are the server's (#121) and join here as their own tool.
// Every result carries the creator's storyboard page, where the member looks and publishes.
import type { McpServer } from '@modelcontextprotocol/server';
import * as z from 'zod';
import { ApiError, createApiClient, createContentApi, type WrappedContent } from '@scutoons/api';
import { lumaPrompt, storyboardKey, type ChapterStoryboard } from '@scutoons/content/storyboard';
import type { Config } from './config.ts';
import { apiMessage, applyOps } from './server.ts';
import type { AppLinks } from './services.ts';
import {
	approveBody,
	checkStoryboard,
	compactGeneration,
	generationTotals,
	locateShot,
	markBody,
	outline,
	recordBody,
	shotIds,
	storyboardHref,
	type Generation,
	type RecordArgs
} from './storyboard-model.ts';
import { shotReferences, studioFolders } from './storyboard-refs.ts';

type Result = { content: { type: 'text'; text: string }[]; isError?: boolean };
// compact JSON: a chapter's outline runs to 60+ shots
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
const write = { readOnlyHint: false, destructiveHint: false, openWorldHint: false };

const toonSchema = z.string().regex(/^[a-z0-9-]{1,64}$/).describe('toon id (list_toons)');
const chapterSchema = z.number().int().min(1).describe('chapter number');
const shotSchema = z.string().regex(/^[\w.-]{1,64}$/).describe('shot id, e.g. "sb-28-2" (read_storyboard lists them)');
const pathSchema = z.array(z.union([z.string(), z.number().int()]));
const opSchema = z.object({ op: z.enum(['set', 'insert', 'remove']), path: pathSchema, value: z.unknown().optional() });

export function registerStoryboardTools(server: McpServer, config: Config, options: { write: boolean; links: AppLinks }): void {
	const client = createApiClient({ baseUrl: config.api, getToken: () => config.token });
	const content = createContentApi(client);
	const open = (toon: string, chapter: number) => storyboardHref(options.links.creator, toon, chapter);

	/** the chapter's storyboard (newest version, the published one, or one by id); a missing one says what exists */
	async function load(toon: string, chapter: number, source: 'draft' | 'published' = 'draft', version?: number): Promise<WrappedContent<ChapterStoryboard>> {
		const key = storyboardKey(toon, chapter);
		try {
			if (version !== undefined) return await content.getVersion<ChapterStoryboard>(key, version);
			return source === 'draft' ? await content.getDraft<ChapterStoryboard>(key) : await content.getPublished<ChapterStoryboard>(key);
		} catch (error) {
			if (!(error instanceof ApiError) || error.status !== 404) throw error;
			const stored = (await content.contentIndex(toon)).entities.filter((e) => e.kind === 'storyboard').map((e) => e.key);
			throw new Error(`no ${source === 'published' && version === undefined ? 'published ' : ''}storyboard ${key} – stored: ${stored.join(', ') || 'none for this toon'}`);
		}
	}
	const versionMeta = (w: WrappedContent<unknown>) => ({ id: w.version.id, status: w.version.status, created_at: w.version.created_at, author: w.version.author_email ?? w.version.author, published: w.published?.id ?? null });

	// ── the document ──

	server.registerTool(
		'read_storyboard',
		{
			description:
				'A chapter\'s storyboard (`storyboard:<toon>:<chapter>`, one document with its scenes inline) in short: scenes → groups → shots (id, format, aspect, status, review, size / angle / move, the cast refs, reading seconds) with each one\'s JSON `path` – for save_draft ops on the same key, or read_entity `path` for the full shot. `scene` / `shot` add that part\'s full JSON; `full` the whole document. `version.id` = the `base` for save_draft. Edits are drafts like every save; the member checks them on the storyboard page (`open`).',
			inputSchema: z.object({
				toon: toonSchema,
				chapter: chapterSchema,
				source: z.enum(['draft', 'published']).optional().describe('draft (default) = the newest version, published = the last published'),
				version: z.number().int().optional().describe('one stored version by id (list_versions)'),
				scene: z.number().int().optional().describe('add this scene\'s full JSON (by scene number)'),
				shot: shotSchema.optional().describe('add this shot\'s full JSON and its scene\'s cast / locations / light'),
				full: z.boolean().optional().describe('add the whole document (long)')
			}),
			annotations: read
		},
		guard(async ({ toon, chapter, source, version, scene, shot, full }) => {
			const w = await load(toon, chapter, source, version);
			const doc = w.content;
			const out: Record<string, unknown> = { key: storyboardKey(toon, chapter), version: versionMeta(w), open: open(toon, chapter), ...outline(doc) };
			if (scene !== undefined) {
				const si = (doc.scenes ?? []).findIndex((s) => s.source?.scene === scene);
				if (si < 0) return fail(`chapter ${chapter} has no scene ${scene} (scenes: ${(doc.scenes ?? []).map((s) => s.source?.scene).join(', ')})`);
				out.scene = { path: ['scenes', si], content: doc.scenes[si] };
			}
			if (shot !== undefined) {
				const found = locateShot(doc, shot);
				if (!found) return fail(`no shot ${shot} in ${storyboardKey(toon, chapter)} (shots: ${shotIds(doc).join(', ')})`);
				const { cast, locations, light } = found.scene;
				out.shot = { path: found.path, content: found.shot, scene: { number: found.scene.source?.scene, cast, locations, light } };
			}
			if (full) out.content = doc;
			return text(out);
		})
	);

	server.registerTool(
		'storyboard_validate',
		{
			description:
				'Checks a chapter\'s storyboard like the storyboard page does (content\'s validateChapter: the format, the vocabulary, the slice grid, ids across scenes; warnings for the rhythm, unsafe formats and open owner reviews) and adds the reading rhythm (summarize: shots, screens, seconds, words per scene and in total). Without `content` / `ops` the stored newest draft is checked; with `ops` that draft with the changes applied (the same ops as save_draft) – check before you save.',
			inputSchema: z.object({
				toon: toonSchema,
				chapter: chapterSchema,
				content: z.unknown().optional().describe('check this whole document instead of the stored one'),
				ops: z.array(opSchema).optional().describe('check the stored draft with these changes applied')
			}),
			annotations: read
		},
		guard(async ({ toon, chapter, content: given, ops }) => {
			if (given !== undefined && ops) return fail('give content or ops, not both');
			let doc: unknown = given;
			let base: number | null = null;
			if (doc === undefined) {
				const w = await load(toon, chapter);
				base = w.version.id;
				doc = ops ? applyOps(w.content, ops) : w.content;
			}
			const r = checkStoryboard(doc);
			return text({ key: storyboardKey(toon, chapter), ...(base !== null ? { base } : {}), errors: r.errors.length, warnings: r.warnings.length, problems: { errors: r.errors, warnings: r.warnings }, rhythm: r.rhythm, open: open(toon, chapter) });
		})
	);

	server.registerTool(
		'storyboard_prompt',
		{
			description:
				'The prompt for Luma for one shot (content\'s lumaPrompt: the story moment, the figures, the camera, the one hard light, the empty zones, the look) from the newest draft, plus the files to attach when they are on this computer (the 3D render as the structure, the style pictures, each figure\'s character image – `missing` says what is not). Nothing is sent anywhere.',
			inputSchema: z.object({ toon: toonSchema, chapter: chapterSchema, shot: shotSchema, source: z.enum(['draft', 'published']).optional() }),
			annotations: read
		},
		guard(async ({ toon, chapter, shot, source }) => {
			const doc = (await load(toon, chapter, source)).content;
			const found = locateShot(doc, shot);
			if (!found) return fail(`no shot ${shot} in ${storyboardKey(toon, chapter)} (shots: ${shotIds(doc).join(', ')})`);
			const refs = shotReferences(studioFolders(), toon, chapter, found.scene, found.shot);
			const any = !!refs.structure || refs.style.length > 0 || refs.characters.length > 0;
			const prompt = lumaPrompt(found.scene, found.shot, any ? { references: { structure: refs.structure ?? undefined, style: refs.style, characters: refs.characters } } : {});
			return {
				content: [
					{ type: 'text', text: prompt },
					{ type: 'text', text: JSON.stringify({ shot, path: found.path, references: refs, open: open(toon, chapter) }) }
				]
			};
		})
	);

	// ── generations: the records (provider jobs come with the server's generators, #121) ──

	server.registerTool(
		'list_generations',
		{
			description:
				'What was generated for a chapter (or one shot), oldest first: id, shot, kind, generator / model, status, the file and the machine it is on (`work` = the shared work storage, `work_path`), the mark (review / rejected), approved, cost, who (`via` = the AI connection). Plus totals (per status / mark, the summed cost). `full` adds prompt, inputs and params.',
			inputSchema: z.object({
				toon: toonSchema,
				chapter: chapterSchema,
				shot: shotSchema.optional(),
				mark: z.enum(['review', 'rejected', 'none']).optional().describe('only these'),
				approved: z.boolean().optional().describe('only approved (true) / not approved (false)'),
				full: z.boolean().optional()
			}),
			annotations: read
		},
		guard(async ({ toon, chapter, shot, mark, approved, full }) => {
			const q = new URLSearchParams({ toon, chapter: String(chapter), ...(shot ? { shot } : {}) });
			const r = await client.get<{ items: Generation[] }>(`/storyboard/generations?${q}`);
			const items = r.items.filter((g) => (mark === undefined || (g.mark ?? 'none') === mark) && (approved === undefined || g.approved === approved));
			return text({ toon, chapter, ...(shot ? { shot } : {}), totals: generationTotals(items), items: full ? items : items.map(compactGeneration), open: open(toon, chapter) });
		})
	);

	if (!options.write) return;

	server.registerTool(
		'mark_generation',
		{
			description:
				'Marks one generation (list_generations gives the id): `review` = put it up for review, `rejected` = out, `none` = clears the mark; an optional note replaces its note. Changes only that record (the API sets marked_at); the file and the storyboard document stay as they are.',
			inputSchema: z.object({ id: z.number().int().min(1), mark: z.enum(['review', 'rejected', 'none']), note: z.string().max(1000).optional() }),
			annotations: { ...write, idempotentHint: true }
		},
		guard(async ({ id, mark, note }) => {
			const { item } = await client.put<{ item: Generation }>(`/storyboard/generations/${id}`, markBody(mark, note));
			return text({ updated: id, item: compactGeneration(item), open: open(item.toon, item.chapter) });
		})
	);

	server.registerTool(
		'approve_generation',
		{
			description:
				'Approves one generation (sets its `approved` flag; `approved: false` takes it back), an optional note replaces its note. Changes only that record – placing the image in the shot (its `source`) is a save_draft on the storyboard, publishing the creator\'s. Only when the member decided it.',
			inputSchema: z.object({ id: z.number().int().min(1), approved: z.boolean().optional().describe('default true'), note: z.string().max(1000).optional() }),
			annotations: { ...write, idempotentHint: true }
		},
		guard(async ({ id, approved, note }) => {
			const { item } = await client.put<{ item: Generation }>(`/storyboard/generations/${id}`, approveBody(approved ?? true, note));
			return text({ updated: id, item: compactGeneration(item), open: open(item.toon, item.chapter) });
		})
	);

	server.registerTool(
		'record_generation',
		{
			description:
				'Adds a record of a generation made on the member\'s machine (their own tools, a local GPU, a manual Luma run): which shot, the file (its name in the workspace folder + sha256) and the machine it is on, the generator / model / prompt / seed / cost. Adds one row to the chapter\'s generations (the budget ledger), signed with this AI connection; it uploads nothing and never starts a paid job. A failed / cancelled run may be recorded too (status, error).',
			inputSchema: z.object({
				toon: toonSchema,
				chapter: chapterSchema,
				shot: shotSchema,
				file: z.string().max(191).describe('the file\'s name in the workspace folder, e.g. "sb-28-2--g3.png"'),
				machine: z.string().max(64).describe('where the file is: the member\'s label for this computer, or "work" (the work storage, then work_path)'),
				generator: z.string().describe('e.g. comfy-local, luma, upload'),
				kind: z.enum(['image', 'video']).optional().describe('default: by the file\'s extension'),
				file_hash: z.string().regex(/^[a-fA-F0-9]{64}$/).optional().describe('sha256 of the file'),
				work_path: z.string().optional().describe('its name in the chapter\'s work storage (must exist there)'),
				take: z.string().max(64).optional(),
				parents: z.array(z.number().int().min(1)).max(100).optional().describe('ids of the generations it was made from'),
				inputs: z.array(z.unknown()).optional().describe('[{ role, key, name? }] – still / control / keyframe / reference / character / style'),
				model: z.string().max(128).optional(),
				mode: z.string().max(64).optional(),
				status: z.enum(['succeeded', 'failed', 'cancelled']).optional().describe('default succeeded'),
				prompt: z.string().max(20000).optional(),
				seed: z.number().int().optional(),
				ratio: z.string().regex(/^[1-9]\d{0,3}:[1-9]\d{0,3}$/).optional(),
				duration: z.number().min(0).optional().describe('seconds (video)'),
				ms: z.number().int().min(0).optional().describe('how long it took'),
				estimate: z.number().min(0).optional(),
				cost: z.number().min(0).optional(),
				currency: z.string().regex(/^[A-Z]{3}$/).optional(),
				mark: z.enum(['review', 'rejected']).optional(),
				note: z.string().max(1000).optional(),
				error: z.string().max(1000).optional(),
				params: z.record(z.string(), z.unknown()).optional()
			}),
			annotations: { ...write, idempotentHint: false }
		},
		guard(async (args) => {
			const { item } = await client.post<{ item: Generation }>('/storyboard/generations', recordBody(args as RecordArgs));
			return text({ recorded: item.id, item: compactGeneration(item), open: open(item.toon, item.chapter) });
		})
	);
}
