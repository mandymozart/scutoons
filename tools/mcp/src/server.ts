// The content tools of the SCUTOONS MCP server (CUTOVER 10.28, agents/MCP.md): tools over the existing content API with
// the member's token – the API's role checks are the only authority, this server adds the creator's validator in front
// of every save. Saves only ever add draft versions (append-only, restorable); publishing is the creator's button
// (agents/MCP.md "Draft → link": no publish tool). `write: false` (an admin who isn't a creator) = the reading tools only.
import { readFile } from 'node:fs/promises';
import type { McpServer } from '@modelcontextprotocol/server';
import * as z from 'zod';
import { ApiError, createApiClient, createAssetsApi, createContentApi, toonRelativePath, type ContentIndexEntity, type WrappedContent } from '@scutoons/api';
import {
	CHARACTERS_COLLECTION,
	MAP_COLLECTION,
	castIndex,
	checkCharacterDocs,
	checkMapDocs,
	collectionDocsFromEntities,
	collectionKinds,
	composeCharacterRegistry,
	clone,
	defaultMoodVocabulary,
	entityKeyFor,
	entityOf,
	expressionIndex,
	getAt,
	hasToonTexts,
	isLocalizedEntity,
	isNeutralDocKey,
	NEUTRAL_DOC_KINDS,
	parseEntityKey,
	resolveText,
	scriptLabels,
	toonLanguages,
	toonName,
	validateCast,
	validateFile,
	validateLocales,
	validateNeutral,
	type CharacterRegistry,
	type CollagePreset,
	type CollectionSpec,
	type Colorations,
	type MoodVocabulary,
	type Path,
	type Problem,
	type ToonCast,
	type ToonIndex
} from '@scutoons/content';
import type { Config } from './config.ts';
import { RULES } from './resources.ts';
import { creatorHref, type AppLinks } from './services.ts';
import { checkStoryboard } from './storyboard-model.ts';

type Json = any;
type Result = { content: { type: 'text'; text: string }[]; isError?: boolean };

const text = (value: unknown): Result => ({
	content: [{ type: 'text', text: typeof value === 'string' ? value : JSON.stringify(value, null, 1) }]
});
const fail = (message: string): Result => ({ ...text(message), isError: true });

/** chapter / season: ONE language-neutral document per entity, texts as locale maps (agents/NEUTRAL.md) */
const isNeutralKind = (kind: string | undefined) => !!kind && (NEUTRAL_DOC_KINDS as readonly string[]).includes(kind);

/** An API error as the agent should read it (409 = someone saved a newer version) */
export function apiMessage(error: unknown): string {
	if (!(error instanceof ApiError)) return error instanceof Error ? error.message : String(error);
	if (error.status === 409)
		return `409 conflict: ${error.message} – a newer version was saved since your base. Read the draft again (read_entity), reapply your change and save with the new version id as base.`;
	if (error.status === 401) return '401: the token does not work any more – expired (six months) or revoked. The member renews it on scutoons.com/account → AI connections (nothing to set up again).';
	if (error.status === 403) return `403: this account may not do that (${error.message}) – the API's role checks decide`;
	if (error.status === 0) return `network error: the API is not reachable (${error.message})`;
	return `${error.status} ${error.code}: ${error.message}`;
}

/** Wraps a tool body: API errors become tool errors the agent can act on */
const guard =
	<A>(fn: (args: A) => Promise<Result>) =>
	async (args: A): Promise<Result> => {
		try {
			return await fn(args);
		} catch (error) {
			return fail(apiMessage(error));
		}
	};

const MIME: Record<string, string> = { webp: 'image/webp', png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', gif: 'image/gif', svg: 'image/svg+xml', avif: 'image/avif', mp4: 'video/mp4', webm: 'video/webm', mp3: 'audio/mpeg', m4a: 'audio/mp4', ogg: 'audio/ogg', wav: 'audio/wav', json: 'application/json', pdf: 'application/pdf' };
const mimeOf = (name: string) => MIME[name.split('.').pop()?.toLowerCase() ?? ''] ?? 'application/octet-stream';

// ── patch operations ─────────────────────────────────────────────────────────────────────────────────────────────────

const pathSchema = z.array(z.union([z.string(), z.number().int()])).describe('JSON path, e.g. ["groups", 5, "events", 0]');
const opSchema = z.object({
	op: z.enum(['set', 'insert', 'remove']).describe('set = replace / create the value at path; insert = into the array at path (last element = index, or "-" for the end); remove = delete the key / array element'),
	path: pathSchema,
	value: z.unknown().optional().describe('set / insert: the JSON value')
});
type Op = z.infer<typeof opSchema>;

export function applyOps(doc: Json, ops: Op[]): Json {
	const out = clone(doc) ?? {};
	ops.forEach((o, i) => {
		const where = `op ${i + 1} (${o.op} ${JSON.stringify(o.path)})`;
		if (!o.path.length) throw new Error(`${where}: empty path`);
		const parent = getAt(out, o.path.slice(0, -1));
		const last = o.path[o.path.length - 1];
		if (o.op === 'insert') {
			if (!Array.isArray(parent)) throw new Error(`${where}: no array at ${JSON.stringify(o.path.slice(0, -1))}`);
			const index = last === '-' ? parent.length : Number(last);
			if (!Number.isInteger(index) || index < 0 || index > parent.length) throw new Error(`${where}: index out of range (0…${parent.length})`);
			if (o.value === undefined) throw new Error(`${where}: value is missing`);
			parent.splice(index, 0, o.value);
		} else if (o.op === 'remove') {
			if (parent === null || typeof parent !== 'object' || !(String(last) in parent)) throw new Error(`${where}: nothing there`);
			if (Array.isArray(parent)) parent.splice(Number(last), 1);
			else delete parent[last];
		} else {
			if (o.value === undefined) throw new Error(`${where}: value is missing`);
			if (parent === null || typeof parent !== 'object') throw new Error(`${where}: no object / array at ${JSON.stringify(o.path.slice(0, -1))}`);
			if (Array.isArray(parent) && !(typeof last === 'number' && last >= 0 && last <= parent.length)) throw new Error(`${where}: index out of range (0…${parent.length})`);
			parent[last] = o.value;
		}
	});
	return out;
}

// ── server ───────────────────────────────────────────────────────────────────────────────────────────────────────────

export function registerContentTools(server: McpServer, config: Config, options: { write: boolean; links: AppLinks }): void {
	const client = createApiClient({ baseUrl: config.api, getToken: () => config.token });
	const content = createContentApi(client);
	const assets = createAssetsApi(client);

	// ── helpers ──

	/** null when nothing is stored (404) */
	async function load(key: string, source: 'draft' | 'published' = 'draft'): Promise<WrappedContent<Json> | null> {
		try {
			return source === 'draft' ? await content.getDraft(key) : await content.getPublished(key);
		} catch (error) {
			if (error instanceof ApiError && error.status === 404) return null;
			throw error;
		}
	}
	const data = async <T>(key: string): Promise<T | null> => ((await load(key)) as WrappedContent<T> | null)?.content ?? null;

	/**
	 * The world map's documents (map, place:<id>, route:<id> – agents/MAP-DOCUMENTS.md), newest or published versions,
	 * in creation order; `replace` checks one document before it is saved
	 */
	async function collection(spec: CollectionSpec, source: 'draft' | 'published' = 'draft', replace?: { key: string; doc: unknown }) {
		const { entities } = await content.withContent(collectionKinds(spec), source);
		return { docs: collectionDocsFromEntities(spec, entities, replace), entities };
	}
	const mapDocs = (source: 'draft' | 'published' = 'draft', replace?: { key: string; doc: unknown }) => collection(MAP_COLLECTION, source, replace);
	/** the character registry composed from `characters` + character:<id> (newest versions), null when nothing is stored */
	async function registry(): Promise<CharacterRegistry | null> {
		const { docs } = await collection(CHARACTERS_COLLECTION);
		return docs.settings || docs.parts.character?.length ? composeCharacterRegistry(docs) : null;
	}
	/**
	 * The custom collage presets a chapter check judges `piece.preset` against (research/COLLAGE.md §12.5, the
	 * creator's judgedPresets): the newest draft's list, none stored = [] (a custom id warns as unknown); a failed load =
	 * undefined, nothing judged – an API that doesn't know the key yet (400) must not stop a chapter's checks
	 */
	async function judgedPresets(): Promise<CollagePreset[] | undefined> {
		try {
			const presets = (await data<Colorations>('colorations'))?.presets;
			return Array.isArray(presets) ? presets : [];
		} catch {
			return undefined;
		}
	}

	async function toonIndex(toon: string): Promise<{ languages: string[]; sourceLanguage: string; [field: string]: Json }> {
		const index = await data<Json>(`toon:${toon}`);
		if (!index) throw new Error(`unknown toon "${toon}" (list_toons)`);
		const languages: string[] = Array.isArray(index.languages) && index.languages.length ? index.languages : ['de'];
		return { ...index, languages, sourceLanguage: index.sourceLanguage ?? languages[0] };
	}

	/**
	 * The stored keys of an entity or key: one key; a chapter / season entity = its one language-neutral
	 * document; every locale of another localized entity (the script; the legacy per-language toon settings)
	 */
	async function keysOf(keyOrEntity: string, locale?: string): Promise<{ entity: string; keys: string[]; primary: string | null }> {
		const ref = parseEntityKey(keyOrEntity);
		if (!ref) throw new Error(`"${keyOrEntity}" is no entity key (scutoons://rules/entity-keys)`);
		const entity = entityOf(keyOrEntity);
		if (!isLocalizedEntity(entity)) return { entity, keys: [entity], primary: null };
		const index = await toonIndex(ref.toon!);
		if (ref.locale || isNeutralKind(ref.kind)) return { entity, keys: [keyOrEntity], primary: index.sourceLanguage };
		const locales = !locale || locale === 'all' ? index.languages : [locale];
		return { entity, keys: locales.map((l) => entityKeyFor(entity, l)), primary: index.sourceLanguage };
	}

	/**
	 * A chapter / season's newest document: the language-neutral one, else (an entity not converted yet –
	 * production before the creator's Convert page) the per-locale one in `locale`
	 */
	async function neutralOrLegacy(entity: string, locale: string, source: 'draft' | 'published' = 'draft'): Promise<{ key: string; doc: WrappedContent<Json> } | null> {
		const neutral = await load(entity, source);
		if (neutral) return { key: entity, doc: neutral };
		const key = entityKeyFor(entity, locale);
		const legacy = await load(key, source);
		return legacy ? { key, doc: legacy } : null;
	}

	const versionMeta = (w: WrappedContent<Json>) => ({
		id: w.version.id,
		status: w.version.status,
		created_at: w.version.created_at,
		author: w.version.author_email ?? w.version.author,
		message: w.version.message,
		published: w.published?.id ?? null
	});

	/**
	 * The creator's checks for one stored document (chapter-editor / meta-page / script-editor); a language-neutral
	 * chapter / season like the creator: `validateNeutral` (the structure once + every language's texts)
	 */
	async function validateDoc(key: string, doc: Json): Promise<{ problems: Problem[]; validated: boolean }> {
		const ref = parseEntityKey(key)!;
		const locale = ref.locale ?? '_';
		const file = entityOf(key);
		const neutral = isNeutralDocKey(key);
		if (ref.kind === 'cast') {
			const [reg, index] = await Promise.all([registry(), toonIndex(ref.toon!)]);
			return { problems: validateCast(doc, { registry: reg, languages: index.languages, file }), validated: true };
		}
		// the world map's documents: the composition with this one in it (references between places and routes), its own problems
		if (ref.kind === 'map' || ref.kind === 'place' || ref.kind === 'route' || ref.kind === 'region') {
			const { docs } = await mapDocs('draft', { key, doc });
			const problems = checkMapDocs(docs).filter((p) => p.key === key);
			return { problems: problems.map(({ level, docPath, message }) => ({ level, file: key, locale: null, path: docPath, message })), validated: true };
		}
		// the character registry's documents: the composition with this one in it (ids, personas, looks), its own problems
		if (ref.kind === 'characters' || ref.kind === 'character') {
			const [{ docs }, moods] = await Promise.all([collection(CHARACTERS_COLLECTION, 'draft', { key, doc }), data<MoodVocabulary>('moods')]);
			const problems = checkCharacterDocs(docs, { languages: ['de', 'en'], moods: moods ?? defaultMoodVocabulary() }).filter((p) => p.key === key);
			return { problems: problems.map(({ level, docPath, message }) => ({ level, file: key, locale: null, path: docPath, message })), validated: true };
		}
		// a chapter's storyboard (#122): the storyboard page's checks – content's validateChapter (warnings include the rhythm)
		if (ref.kind === 'storyboard') {
			const { errors, warnings } = checkStoryboard(doc);
			const problem = (level: 'error' | 'warning') => (message: string): Problem => ({ level, file: key, locale: null, path: [], message });
			return { problems: [...errors.map(problem('error')), ...warnings.map(problem('warning'))], validated: true };
		}
		const kind = ({ chapter: 'chapter', season: 'season', 'toon-settings': 'toon', toon: 'index', 'season-script': 'season-script', 'chapter-script': 'chapter-script', kanon: 'kanon', colorations: 'colorations' } as Record<string, string>)[ref.kind];
		// the toon entry: its language-neutral settings + its texts per language (agents/NEUTRAL.md step 4)
		if (kind === 'index') {
			const { source, languages } = toonLanguages(doc as ToonIndex);
			return { problems: validateNeutral('index', doc, { source, languages }, { file }), validated: true };
		}
		if (!kind) return { problems: [], validated: false };
		const ctx: Parameters<typeof validateFile>[3] = { file };
		if (kind === 'chapter') {
			const [cast, reg, moods, presets] = await Promise.all([data<ToonCast>(`cast:${ref.toon}`), registry(), data<MoodVocabulary>('moods'), judgedPresets()]);
			const chapter = doc;
			// like the creator's chapter editor: the cast of season 1, the registry's looks, the mood vocabulary, the custom collage presets
			if (cast) ctx.cast = castIndex(cast, 1);
			if (reg) ctx.expressions = expressionIndex(reg);
			ctx.moods = (moods ?? defaultMoodVocabulary()).moods.map((m) => m.id);
			if (presets) ctx.presets = presets;
			ctx.audio = chapter?.audio;
		}
		if (neutral) {
			const index = await toonIndex(ref.toon!);
			return { problems: validateNeutral(kind, doc, { source: index.sourceLanguage, languages: index.languages }, ctx), validated: true };
		}
		return { problems: validateFile(kind, doc, locale, ctx), validated: true };
	}

	const summary = (problems: Problem[]) => ({
		errors: problems.filter((p) => p.level === 'error').length,
		warnings: problems.filter((p) => p.level === 'warning').length,
		problems: problems.map((p) => ({ level: p.level, locale: p.locale, path: p.path, message: p.message }))
	});

	// ── resources ──

	for (const rule of RULES) {
		server.registerResource(rule.name, rule.uri, { title: rule.title, description: rule.description, mimeType: 'text/markdown' }, async (uri) => ({
			contents: [{ uri: uri.href, mimeType: 'text/markdown', text: rule.text() }]
		}));
	}

	// ── tools: reading ──

	const read = { readOnlyHint: true, openWorldHint: false };

	server.registerTool(
		'list_toons',
		{
			description: 'Every toon with its names, languages (the first source language is the primary), status and how many documents it has.',
			inputSchema: z.object({}),
			annotations: read
		},
		guard(async () => {
			const all = await client.get<{ entities: ContentIndexEntity[] }>('/entities');
			const toons = all.entities.filter((e) => e.kind === 'toon');
			const out = [];
			for (const t of toons) {
				const index = await data<Json>(t.key);
				out.push({
					id: t.toon,
					// the name = the entry's `title` per language (resolved; older entries: the old i18n names)
					names: Object.fromEntries(toonLanguages(index).languages.map((l) => [l, toonName(index, l)])),
					languages: index?.languages,
					sourceLanguage: index?.sourceLanguage,
					status: index?.status,
					// the toon settings per language (read_entity toon:<toon> for all of them)
					...(hasToonTexts(index) ? { title: index?.title, languageStatus: index?.languageStatus } : { settings: 'per language (toon-settings, legacy – not converted yet)' }),
					documents: all.entities.filter((e) => e.toon === t.toon).length
				});
			}
			return text(out);
		})
	);

	server.registerTool(
		'list_chapters',
		{
			description:
				'The chapters of a toon (seasons, chapter numbers, titles, status from the season in `locale`) with their groups: index, label, slice count and the script points (label names a divert can go to). A jumpTo targets a group by its label. Without `chapter` every chapter is listed. `key` = the document to read / save (language-neutral: `chapter:<toon>:<n>`).',
			inputSchema: z.object({
				toon: z.string(),
				chapter: z.number().int().min(0).optional().describe('only this chapter'),
				locale: z.string().optional().describe('language of titles / labels (default: the toon\'s source language)')
			}),
			annotations: read
		},
		guard(async ({ toon, chapter, locale }) => {
			const index = await toonIndex(toon);
			const lang = locale ?? index.sourceLanguage;
			const chain = [lang, index.sourceLanguage, ...index.languages];
			const stored = (await content.contentIndex(toon)).entities;
			const numbers = (kind: string) =>
				[...new Set(stored.map((e) => parseEntityKey(e.key)).filter((r) => r?.kind === kind).map((r) => r!.number!))].sort((a, b) => a - b);
			// a chapter's status = its entry in a season's sequence (per language)
			const statusOf = new Map<number, unknown>();
			for (const n of numbers('season')) {
				const season = await neutralOrLegacy(`season:${toon}:${n}`, lang);
				for (const item of (season?.doc.content?.sequence ?? []) as Json[]) {
					if (item?.type === 'chapter' && !statusOf.has(Number(item.number))) statusOf.set(Number(item.number), resolveText(item.status, chain));
				}
			}
			const chapters = [];
			for (const n of numbers('chapter')) {
				if (chapter !== undefined && n !== chapter) continue;
				const found = await neutralOrLegacy(`chapter:${toon}:${n}`, lang);
				if (!found) {
					chapters.push({ number: n, key: `chapter:${toon}:${n}`, missing: true });
					continue;
				}
				const doc = found.doc;
				const groups = [];
				for (const [i, g] of ((doc.content?.groups ?? []) as Json[]).entries()) {
					groups.push({
						index: i,
						label: g?.label ?? null,
						slices: Array.isArray(g?.slices) ? g.slices.length : 0,
						events: Array.isArray(g?.events) ? g.events.length : 0,
						points: scriptLabels(g?.events)
					});
				}
				chapters.push({
					number: n,
					key: found.key,
					...(isNeutralDocKey(found.key) ? {} : { legacy: 'stored per language (not converted yet) – read only' }),
					title: resolveText(doc.content?.title, chain),
					status: statusOf.get(n) ?? null,
					version: versionMeta(doc),
					groups
				});
			}
			return text({ toon, locale: lang, languages: index.languages, seasons: numbers('season'), chapters });
		})
	);

	server.registerTool(
		'read_entity',
		{
			description:
				'Reads a stored document by entity key (scutoons://rules/entity-keys). A chapter / season (`chapter:catpope:2`) is ONE language-neutral document: structure once, every text a locale map { "de": …, "en": … }; so is the toon entry `toon:catpope` with the toon\'s settings (title / description / release / languageStatus per language). The script without locale returns every language, a full key one. Each result has `version.id` – pass it as `base` to save_draft. `source`: draft = the newest version (draft or published), published = what readers see, or one old version by id. `path` returns only that part of the content; `lines` [from, to] (1-based) only those lines of a text (e.g. the screenplay: path ["text"], line numbers from search_content).',
			inputSchema: z.object({
				key: z.string().describe('entity (`chapter:catpope:2`, `season-script:catpope:1`, `kanon:character.catpope`) or stored key (`season-script:catpope:1:de`)'),
				source: z.enum(['draft', 'published']).optional().describe('default draft'),
				version: z.number().int().optional().describe('one stored version by id (overrides source; needs a full key)'),
				locale: z.string().optional().describe('the script: one language, or "all" (default); not for language-neutral documents'),
				path: pathSchema.optional(),
				lines: z.tuple([z.number().int().min(1), z.number().int().min(1)]).optional().describe('[from, to] lines of a text value')
			}),
			annotations: read
		},
		guard(async ({ key, source, version, locale, path, lines }) => {
			const { keys } = await keysOf(key, locale);
			// a chapter / season not converted yet (production before the creator's Convert page): its per-locale documents, read only
			if (keys.length === 1 && isNeutralDocKey(keys[0]) && version === undefined && !(await load(keys[0], source ?? 'draft'))) {
				const legacy = (await toonIndex(parseEntityKey(keys[0])!.toon!)).languages.map((l) => entityKeyFor(keys[0], l));
				if ((await Promise.all(legacy.map((k) => load(k, source ?? 'draft')))).some((w) => w)) keys.splice(0, 1, ...legacy);
			}
			const out = [];
			for (const k of keys) {
				const w = version !== undefined ? await content.getVersion<Json>(k, version) : await load(k, source ?? 'draft');
				if (!w) {
					out.push({ key: k, missing: true });
					continue;
				}
				if (isNeutralKind(parseEntityKey(k)?.kind) && !isNeutralDocKey(k)) {
					out.push({ key: k, legacy: 'stored per language, not converted yet – read only; save_draft writes the language-neutral entity', version: versionMeta(w), content: path ? getAt(w.content, path) : w.content });
					continue;
				}
				if (parseEntityKey(k)?.kind === 'toon-settings') {
					const toonKey = `toon:${parseEntityKey(k)!.toon}`;
					out.push({ key: k, legacy: `the old per-language toon settings – read only; the toon entry ${toonKey} holds them (title / description / release / languageStatus per language) – save that`, version: versionMeta(w), content: path ? getAt(w.content, path) : w.content });
					continue;
				}
				let value = path ? getAt(w.content, path) : w.content;
				if (lines && typeof value === 'string') {
					const all = value.split(/\r?\n/);
					value = all.slice(lines[0] - 1, lines[1]).map((l, i) => `${lines[0] + i}: ${l}`).join('\n');
				}
				out.push({ key: k, version: versionMeta(w), content: value });
			}
			return text(out.length === 1 ? out[0] : out);
		})
	);

	server.registerTool(
		'list_entities',
		{
			description:
				'Every stored document of a toon (or the platform-wide ones with toon ""): key, kind, language, versions, newest / published version. The map of what exists – screenplay (script), seasons, chapters, scenes, cast, credits, settings.',
			inputSchema: z.object({ toon: z.string().describe('toon id, "" = platform-wide (characters, moods, map …)') }),
			annotations: read
		},
		guard(async ({ toon }) => {
			const entities = (await content.contentIndex(toon)).entities;
			return text(entities.map((e) => ({ key: e.key, kind: e.kind, locale: e.locale, versions: e.versions, latest: e.latest, published: e.published })));
		})
	);

	server.registerTool(
		'search_content',
		{
			description:
				'Full-text search (case-insensitive) in the newest drafts of a toon\'s documents – the screenplay (script, per line), seasons, chapters, settings, cast, credits – or of the platform-wide ones (characters, moods, map) with toon "". Returns the document key, where (JSON path, screenplay line) and a snippet; read more with read_entity (path / lines). The source language holds the complete content – translations can be incomplete.',
			inputSchema: z.object({
				query: z.string().min(2),
				toon: z.string().describe('toon id, "" = platform-wide documents'),
				kinds: z.array(z.string()).optional().describe('only these kinds, e.g. ["season-script", "kanon"]'),
				locale: z.string().optional().describe('only this language'),
				limit: z.number().int().min(1).max(100).optional().describe('default 30')
			}),
			annotations: read
		},
		guard(async ({ query, toon, kinds, locale, limit }) => {
			const needle = query.toLowerCase();
			const max = limit ?? 30;
			const stored = (await content.contentIndex(toon)).entities;
			// a per-locale chapter / season behind its language-neutral document is not what anyone reads
			const neutral = new Set(stored.filter((e) => isNeutralDocKey(e.key)).map((e) => e.key));
			const entities = stored.filter(
				(e) => (!kinds || kinds.includes(e.kind)) && (!locale || e.locale === locale || e.locale === null) && !(e.locale && neutral.has(entityOf(e.key)))
			);
			// with `locale`: in a language-neutral document only that language's texts (a locale map's other entries are skipped)
			const others = locale && toon ? (await toonIndex(toon)).languages.filter((l) => l !== locale) : [];
			const hits: { key: string; where: string; snippet: string }[] = [];
			const snippet = (s: string) => {
				const at = s.toLowerCase().indexOf(needle);
				return (at > 60 ? '…' : '') + s.slice(Math.max(0, at - 60), at + needle.length + 100).trim() + (s.length > at + needle.length + 100 ? '…' : '');
			};
			const walk = (key: string, value: Json, at: Path) => {
				if (hits.length >= max) return;
				if (typeof value === 'string') {
					if (!value.toLowerCase().includes(needle)) return;
					// long texts (the screenplay): one hit per line, with its number for read_entity `lines`
					if (value.includes('\n'))
						value.split(/\r?\n/).forEach((l, i) => {
							if (hits.length < max && l.toLowerCase().includes(needle)) hits.push({ key, where: `${JSON.stringify(at)} line ${i + 1}`, snippet: snippet(l) });
						});
					else hits.push({ key, where: JSON.stringify(at), snippet: snippet(value) });
				} else if (value && typeof value === 'object')
					for (const [k, v] of Object.entries(value)) if (Array.isArray(value) || !(isNeutralDocKey(key) && others.includes(k))) walk(key, v, [...at, Array.isArray(value) ? Number(k) : k]);
			};
			for (const e of entities) {
				if (hits.length >= max) break;
				walk(e.key, await data<Json>(e.key), []);
			}
			return text({ searched: entities.length, hits, ...(hits.length >= max ? { more: 'limit reached – narrow with kinds / locale or raise limit' } : {}) });
		})
	);

	server.registerTool(
		'read_characters',
		{
			description:
				'The characters a scene can use: the toon\'s cast (character → personas, season 1 like the creator), the registry\'s looks and the moods each look has a portrait for, and the mood vocabulary. joinScene / showMessage reference these ids.',
			inputSchema: z.object({ toon: z.string() }),
			annotations: read
		},
		guard(async ({ toon }) => {
			const [cast, reg, moods] = await Promise.all([data<ToonCast>(`cast:${toon}`), registry(), data<MoodVocabulary>('moods')]);
			const casted = cast ? castIndex(cast, 1) : null;
			const expressions = reg ? expressionIndex(reg) : {};
			const names = Object.fromEntries((reg?.characters ?? []).map((c) => [c.id, c.name]));
			const ids = casted ? Object.keys(casted) : Object.keys(expressions);
			return text({
				cast: casted ? 'season 1' : 'no cast stored – every registry character',
				characters: ids.map((id) => ({ id, name: names[id], personas: casted?.[id], looks: expressions[id] ?? null })),
				moods: (moods ?? defaultMoodVocabulary()).moods.map((m) => ({ id: m.id, label: m.label }))
			});
		})
	);

	server.registerTool(
		'read_map_places',
		{
			description:
				'Places of the SCUTOONS world map – each place is its own document `place:<id>` (routes `route:<id>`, the size / layers of the map are `map`; read_entity / save_draft / validate work on those keys, the document of a place is the place without its id). Here: id, key, name, position, which toons use it, hidden, linked chapter, whether a newer draft waits or it is removed (a tombstone: `removed: true` – the id stays taken).',
			inputSchema: z.object({
				toon: z.string().optional().describe('only places this toon uses'),
				published: z.boolean().optional().describe('what readers see (default: the newest drafts)')
			}),
			annotations: read
		},
		guard(async ({ toon, published }) => {
			const { docs, entities } = await mapDocs(published ? 'published' : 'draft');
			if (!docs.settings) return text('no world map stored');
			const meta = new Map(entities.map((e) => [e.key, e]));
			return text(
				(docs.parts.place ?? [])
					.filter((p) => !toon || p.doc.toons?.includes(toon))
					.map(({ id, doc }) => {
						const e = meta.get(`place:${id}`);
						const { name, x, y, toons, hidden, chapter, removed } = doc;
						return { id, key: `place:${id}`, name, x, y, toons, hidden, chapter, ...(removed ? { removed } : {}), draft: !!e && e.latest !== e.published };
					})
			);
		})
	);

	server.registerTool(
		'search_assets',
		{
			description: 'Finds media in the asset manager (images, video, audio). `path` is what content stores (CDN root; in a toon\'s documents relative to the toon folder, e.g. "s1/c2/x.webp" for "catpope/s1/c2/x.webp"). Renditions = quality variants.',
			inputSchema: z.object({
				q: z.string().optional().describe('search text (slug, title, tags, path)'),
				folder: z.string().optional(),
				kind: z.enum(['image', 'video', 'audio', 'archive', 'document', 'other']).optional(),
				page: z.number().int().min(1).optional(),
				page_size: z.number().int().min(1).max(100).optional().describe('default 30'),
				tiles: z.enum(['hide', 'only', 'all']).optional().describe('map tiles (<set>/<z>/<x>_<y> below a map/ folder, thousands): hidden by default')
			}),
			annotations: read
		},
		guard(async ({ q, folder, kind, page, page_size, tiles }) => {
			const r = await assets.list({ q, folder, kind, page, page_size: page_size ?? 30, sort: 'slug', tiles });
			return text({
				total: r.pagination.total,
				page: r.pagination.page,
				items: r.items.map((a) => ({
					id: a.id,
					slug: a.slug,
					kind: a.kind,
					title: a.title,
					tags: a.tags,
					path: a.path,
					width: a.width,
					height: a.height,
					duration: a.duration,
					usages: a.usages,
					renditions: a.renditions.map((x) => ({ label: x.label, quality: x.quality, path: x.path, width: x.width, height: x.height }))
				}))
			});
		})
	);

	server.registerTool(
		'list_versions',
		{
			description: 'The version history of a stored key (newest first): id, draft / published, author, message. Every save is its own version.',
			inputSchema: z.object({ key: z.string().describe('stored key (with locale for localized kinds)'), limit: z.number().int().min(1).max(200).optional() }),
			annotations: read
		},
		guard(async ({ key, limit }) => {
			const versions = (await content.listVersions(key)).versions.slice(0, limit ?? 20);
			return text(versions.map((v) => ({ id: v.id, status: v.status, created_at: v.created_at, author: v.author_email ?? v.author, message: v.message, size: v.size })));
		})
	);

	// ── tools: checking ──

	server.registerTool(
		'validate',
		{
			description:
				'Runs the creator\'s checks (shared/content validate.ts: structure, VN events, script points, flow, media) on a document. Without `content` / `ops` the stored draft is checked. A chapter / season / the toon entry (one language-neutral document) is checked once plus the texts of every language (missing or too long translations carry their locale); the script without locale checks every language and compares them.',
			inputSchema: z.object({
				key: z.string(),
				content: z.unknown().optional().describe('check this content instead of the stored draft (one document: a language-neutral entity or a full key)'),
				ops: z.array(opSchema).optional().describe('check the stored draft with these changes applied (one document)')
			}),
			annotations: read
		},
		guard(async ({ key, content: given, ops }) => {
			const { entity, keys, primary } = await keysOf(key);
			if ((given !== undefined || ops) && keys.length !== 1) return fail('content / ops need one document (a full key with locale for the script)');
			const docs: Record<string, Json> = {};
			const problems: Problem[] = [];
			let validated = true;
			for (const k of keys) {
				let doc = given !== undefined ? given : await data<Json>(k);
				if (ops) doc = applyOps(doc, ops);
				docs[parseEntityKey(k)?.locale ?? '_'] = doc;
				if (doc == null) {
					problems.push({ level: 'warning', file: entity, locale: parseEntityKey(k)?.locale ?? null, path: [], message: `nothing stored at ${k}` });
					continue;
				}
				const r = await validateDoc(k, doc);
				validated &&= r.validated;
				problems.push(...r.problems);
			}
			const kind = parseEntityKey(entity)!.kind;
			if (keys.length > 1 && primary) problems.push(...validateLocales(kind, entity, docs, primary));
			return text(validated ? summary(problems) : { note: `no checks for ${kind} documents (stored as they are)`, ...summary(problems) });
		})
	);

	// ── tools: writing (creators; an admin who isn't one reads only) ──

	if (!options.write) return;

	server.registerTool(
		'upload_asset',
		{
			description:
				'Puts a NEW file into the asset manager (CDN) so content can use it – e.g. an image another AI service made. Source: an http(s) `url` or a local `file`. `path` = CDN-root path incl. the toon folder (e.g. "catpope/s1/c2/s1c2-slice-31.webp"; see scutoons://rules/slices for sizes). Never overwrites: a taken path → 409. Returns the path to store in the toon\'s documents (relative to the toon folder).',
			inputSchema: z.object({
				url: z.string().url().optional(),
				file: z.string().optional().describe('local file path (stdio server only)'),
				path: z.string().describe('CDN-root target path'),
				title: z.string().optional(),
				tags: z.array(z.string()).optional()
			}),
			annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: false, openWorldHint: true }
		},
		guard(async ({ url, file, path, title, tags }) => {
			if (!url === !file) return fail('give either url or file');
			let blob: Blob;
			if (url) {
				if (!/^https?:\/\//i.test(url)) return fail('url must be http(s)');
				const res = await fetch(url);
				if (!res.ok) return fail(`download failed: ${res.status} ${res.statusText}`);
				blob = new Blob([await res.arrayBuffer()], { type: res.headers.get('content-type')?.split(';')[0] || mimeOf(path) });
			} else {
				blob = new Blob([await readFile(file!)], { type: mimeOf(file!) });
			}
			const r = await assets.upload(blob, path, { title, tags });
			const toon = path.split('/')[0];
			return text({ uploaded: r.uploaded, asset: r.asset.id, kind: r.asset.kind, contentPath: toonRelativePath(r.uploaded, toon), size: blob.size });
		})
	);

	server.registerTool(
		'upload_score_file',
		{
			description:
				"Puts a score's MusicXML or PDF into the PROTECTED storage (the vault, never the public CDN; agents/research/SCORES.md §5) for the catalog item `item` (its id, e.g. \"cloud9_en\"). Source: a local `file` (stdio server only) or an http(s) `url`. Returns the path for the item's `score.musicxml` / `score.pdf` (`scores/<item>/<hash>.<ext>`); readers only ever get signed links. Checked: a real PDF / MusicXML.",
			inputSchema: z.object({
				item: z.string().regex(/^[a-z0-9_]{1,100}$/).describe('catalog item id'),
				kind: z.enum(['musicxml', 'pdf']),
				file: z.string().optional().describe('local file path (stdio server only)'),
				url: z.string().url().optional()
			}),
			annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: true, openWorldHint: true }
		},
		guard(async ({ item, kind, file, url }) => {
			if (!url === !file) return fail('give either url or file');
			let blob: Blob;
			if (url) {
				if (!/^https?:\/\//i.test(url)) return fail('url must be http(s)');
				const res = await fetch(url);
				if (!res.ok) return fail(`download failed: ${res.status} ${res.statusText}`);
				blob = new Blob([await res.arrayBuffer()]);
			} else {
				blob = new Blob([await readFile(file!)]);
			}
			const r = await assets.uploadProtected(blob, item, kind);
			return text({ path: r.path, size: blob.size });
		})
	);

	server.registerTool(
		'upload_album_file',
		{
			description:
				"Puts a physical release's audio into the PROTECTED storage (the vault, never the public CDN; owner 2026-10-14: release audio protected, game audio public) for the catalog album `album` (its id, e.g. \"awakening\"): `kind` track = an MP3 (a song, or one of its quality files 96k / 128k / 320k), download = the album ZIP. Source: a local `file` (stdio server only) or an http(s) `url`. Returns the path (`albums/<album>/<hash>.<ext>` – the same bytes give the same path) for the album's `tracks[i].src` / `tracks[i].variants.<level>` / `download` – set it in the album document with save_draft (`album:<id>`; a person publishes it in the creator). Readers only ever get signed links. Checked: a real MP3 / ZIP.",
			inputSchema: z.object({
				album: z.string().regex(/^[a-z0-9_]{1,100}$/).describe('catalog album id'),
				kind: z.enum(['track', 'download']),
				file: z.string().optional().describe('local file path (stdio server only)'),
				url: z.string().url().optional()
			}),
			annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: true, openWorldHint: true }
		},
		guard(async ({ album, kind, file, url }) => {
			if (!url === !file) return fail('give either url or file');
			let blob: Blob;
			if (url) {
				if (!/^https?:\/\//i.test(url)) return fail('url must be http(s)');
				const res = await fetch(url);
				if (!res.ok) return fail(`download failed: ${res.status} ${res.statusText}`);
				blob = new Blob([await res.arrayBuffer()]);
			} else {
				blob = new Blob([await readFile(file!)]);
			}
			// AssetsController::uploadProtected (album=…&file=track|download)
			const r = await client.put<{ path: string }>(`/assets/protected/?album=${encodeURIComponent(album)}&file=${kind}`, blob);
			return text({ path: r.path, size: blob.size });
		})
	);

	server.registerTool(
		'save_draft',
		{
			description:
				'Saves a NEW draft version of one stored key (append-only: nothing is overwritten, every older version stays restorable; readers keep seeing the published version). Give `ops` (changes applied to the version `base`) or the full `content`. `base` = the version id you read (read_entity) – a newer save in between → 409 conflict. A chapter / season is saved as its one language-neutral document (`chapter:catpope:2`; a text is { "de": …, "en": … } – set one language at e.g. ["events", 3, "params", "text", "en"]); the toon settings are the toon entry `toon:catpope` (["title", "en"], ["languageStatus", "en"] …). The creator\'s checks run first: errors refuse the save (unless allow_errors), warnings are returned.',
			inputSchema: z.object({
				key: z.string().describe('stored key: chapter / season / toon entry without locale (`chapter:catpope:2`, `toon:catpope`); a script / Kanon text with it (`season-script:catpope:1:de`, `kanon:place.cathedral:en`)'),
				base: z.number().int().nullable().describe('the version id the change starts from (null = nothing stored yet)'),
				ops: z.array(opSchema).optional(),
				content: z.unknown().optional().describe('the whole document (instead of ops)'),
				message: z.string().max(255).optional().describe('what changed (shown in the version history)'),
				allow_errors: z.boolean().optional().describe('save even with validation errors (default false)')
			}),
			annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: false, openWorldHint: false }
		},
		guard(async ({ key, base, ops, content: given, message, allow_errors }) => {
			const ref = parseEntityKey(key);
			if (!ref) return fail(`"${key}" is no entity key`);
			if (isNeutralKind(ref.kind) && ref.locale)
				return fail(`chapters and seasons are one language-neutral document – save ${entityOf(key)} (texts are locale maps { "de": …, "en": … }; scutoons://rules/workflow)`);
			if (ref.kind === 'toon-settings')
				return fail(`the toon settings live in the toon entry – save toon:${ref.toon} (title / description / release / languageStatus are locale maps { "de": …, "en": … }; scutoons://rules/workflow)`);
			if (!isNeutralKind(ref.kind) && isLocalizedEntity(key) && !ref.locale) return fail(`${key} is stored per language – save one locale (${key}:<locale>)`);
			if ((given === undefined) === !ops) return fail('give either ops or content');
			let doc = given;
			if (ops) {
				if (base === null) return fail('ops need a base version');
				const w = await content.getVersion<Json>(key, base);
				doc = applyOps(w.content, ops);
			}
			const checked = await validateDoc(key, doc);
			const s = summary(checked.problems);
			if (s.errors && !allow_errors) return { ...text({ saved: false, reason: 'validation errors – fix them (or allow_errors: true)', ...s }), isError: true };
			// a personal token: the API signs the save "(AI: <connection name>)"; a copied session (local dev) gets "(agent)"
			const note = message || 'AI edit';
			const r = await content.putDraft(key, doc, base, config.token.startsWith('sct_') ? note : `${note} (agent)`);
			return text({ saved: !r.unchanged, unchanged: r.unchanged, version: r.version.id, status: r.version.status, open: creatorHref(options.links, key), ...s });
		})
	);

	server.registerTool(
		'restore_version',
		{
			description: 'Brings an older version back: its content is saved as a NEW draft version (append-only; the versions in between stay). Use list_versions to find the id.',
			inputSchema: z.object({ key: z.string(), version: z.number().int(), message: z.string().max(255).optional() }),
			annotations: { readOnlyHint: false, destructiveHint: false, openWorldHint: false }
		},
		guard(async ({ key, version, message }) => {
			const r = await content.restoreVersion(key, version, message);
			return text({ restored: version, version: r.version.id, status: r.version.status, open: creatorHref(options.links, key) });
		})
	);

	server.registerTool(
		'submit_for_review',
		{
			description:
				'Hands saved drafts to the toon\'s reviewers (review before publish): each item pinned to a version (default: the newest), an optional note; the reviewers get a mail and publish or send back. For creators who need a review (publishing answers 403 review_required for them). Only when the creator asks for it. 409 no_reviewer = nobody reviews that toon – an admin has to add one.',
			inputSchema: z.object({
				items: z.array(z.object({ key: z.string().describe('stored key'), version: z.number().int().optional() })).min(1),
				note: z.string().max(1000).optional()
			}),
			annotations: { readOnlyHint: false, destructiveHint: false, openWorldHint: true }
		},
		guard(async ({ items, note }) => {
			const r = await client.post<{ reviews: Json[]; reviewers: string[] }>('/reviews', { items, note });
			return text({ submitted: r.reviews?.length ?? items.length, reviewers: r.reviewers });
		})
	);
}
