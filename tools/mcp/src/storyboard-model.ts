// The pure parts of the storyboard tools (storyboard.ts, #122): the outline an AI reads instead of the whole chapter
// document, a shot's JSON path for save_draft ops, the checks + rhythm, and the shapes of the generation records the
// API takes and answers (server/docs/API.md "Storyboard"). The storyboard's own logic stays in
// @scutoons/content/storyboard (validateChapter, summarize, measureShot, kindOf …) – used here, never copied.
import { kindOf, measureShot, summarize, validateChapter, type ChapterStoryboard, type Shot, type StoryboardDoc } from '@scutoons/content/storyboard';

export type JsonPath = (string | number)[];

/** the creator's storyboard page of a chapter (creator `routes/[toonId]/chapter/[chapterNumber]/storyboard`) */
export const storyboardHref = (creator: string, toon: string, chapter: number): string => `${creator}/${toon}/chapter/${chapter}/storyboard`;

const round = (n: number, digits = 1) => Math.round(n * 10 ** digits) / 10 ** digits;
/** drops undefined / null / empty-array fields (the outline stays short) */
const compact = <T extends Record<string, unknown>>(o: T): Partial<T> =>
	Object.fromEntries(Object.entries(o).filter(([, v]) => v !== undefined && v !== null && !(Array.isArray(v) && !v.length))) as Partial<T>;

/** one shot as the outline lists it: what it is, where it stands, the path to edit it */
export function shotLine(shot: Shot, path: JsonPath) {
	const m = measureShot(shot);
	return compact({
		id: shot.id,
		path,
		format: shot.format,
		aspect: shot.aspect,
		status: shot.status,
		review: shot.review ? compact({ status: shot.review.status, note: shot.review.note }) : undefined,
		size: shot.shot?.size,
		angle: shot.shot?.angle,
		move: shot.move,
		figures: (shot.figures ?? []).map((f) => f.cast),
		dialog: shot.dialog?.length || undefined,
		source: shot.source?.type || shot.source?.want ? compact({ type: shot.source?.type, want: shot.source?.want }) : undefined,
		title: shot.title || undefined,
		seconds: round(m.seconds)
	});
}

/**
 * The chapter document in short: scenes → groups → shots (id, format, aspect, status, review, the cast refs, the JSON
 * path for save_draft ops / read_entity `path`), plus the counts. The whole JSON stays one read_entity away.
 */
export function outline(doc: ChapterStoryboard) {
	const scenes = (doc.scenes ?? []).map((sb, si) => ({
		scene: sb.source?.scene ?? null,
		heading: sb.source?.heading ?? null,
		path: ['scenes', si] as JsonPath,
		cast: (sb.cast ?? []).map((c) => compact({ ref: c.ref, character: c.character, persona: c.persona, new: c.new })),
		groups: (sb.groups ?? []).map((g, gi) =>
			compact({
				label: g.label,
				renderer: g.renderer?.type,
				events: g.events?.length || undefined,
				shots: (g.shots ?? []).map((shot, k) => shotLine(shot, ['scenes', si, 'groups', gi, 'shots', k]))
			})
		),
		...(sb.notes?.length ? { notes: sb.notes.length } : {})
	}));
	const shots = scenes.reduce((n, s) => n + s.groups.reduce((m, g) => m + (g.shots?.length ?? 0), 0), 0);
	return { toon: doc.toon, chapter: doc.chapter, scenes: scenes.length, shots, outline: scenes };
}

/** a shot by id with its scene and its JSON path in the chapter document; null = no such shot */
export function locateShot(doc: ChapterStoryboard, id: string): { scene: StoryboardDoc; shot: Shot; path: JsonPath } | null {
	for (const [si, scene] of (doc.scenes ?? []).entries())
		for (const [gi, g] of (scene.groups ?? []).entries())
			for (const [k, shot] of (g.shots ?? []).entries()) if (shot.id === id) return { scene, shot, path: ['scenes', si, 'groups', gi, 'shots', k] };
	return null;
}

/** every shot id of the document (for "no such shot – these exist") */
export const shotIds = (doc: ChapterStoryboard): string[] => (doc.scenes ?? []).flatMap((sb) => (sb.groups ?? []).flatMap((g) => (g.shots ?? []).map((s) => s.id)));

/** the reading rhythm (summarize): per scene and in total – screens (1 decimal), seconds, words */
export function rhythm(doc: ChapterStoryboard) {
	const { scenes, total } = summarize((doc.scenes ?? []).map((sb) => ({ sb })));
	const shape = <T extends { screens: number; seconds: number }>(s: T) => ({ ...s, screens: round(s.screens), seconds: Math.round(s.seconds) });
	return { scenes: scenes.map(shape), total: shape(total) };
}

/** validateChapter's errors / warnings + the rhythm (a document that isn't a storyboard at all gets errors, no rhythm) */
export function checkStoryboard(doc: unknown) {
	if (!doc || typeof doc !== 'object') return { errors: ['chapter: no storyboard document'], warnings: [] as string[] };
	const d = doc as ChapterStoryboard;
	const { errors, warnings } = validateChapter(d);
	return { errors, warnings, ...(Array.isArray(d.scenes) ? { rhythm: rhythm(d) } : {}) };
}

// ── generations (server Domain\Storyboard\Generations) ───────────────────────────────────────────────────────────────

/** a stored generation as the API answers it (the fields the tools read) */
export interface Generation {
	id: number;
	toon: string;
	chapter: number;
	shot: string;
	kind: 'image' | 'video';
	status: string;
	[field: string]: unknown;
}

/** the long fields list_generations leaves out unless `full` (prompt, inputs, params …) */
const LONG = ['prompt', 'inputs', 'params', 'toon', 'chapter', 'created_by', 'updated_at', 'marked_at', 'file_hash', 'work_url'];

/** one record in short: what, which file where, the review, the cost, who */
export function compactGeneration(g: Generation): Record<string, unknown> {
	const out = Object.fromEntries(Object.entries(g).filter(([k, v]) => !LONG.includes(k) && v !== null && v !== false && !(Array.isArray(v) && !v.length)));
	if (typeof g.prompt === 'string') out.prompt = g.prompt.length > 80 ? `${g.prompt.slice(0, 80)}…` : g.prompt;
	return out;
}

/** what a list of generations comes to: per status, marks, approved, the summed cost per currency */
export function generationTotals(items: Generation[]) {
	const count = (f: (g: Generation) => unknown) => items.reduce<Record<string, number>>((a, g) => { const k = String(f(g) ?? 'none'); a[k] = (a[k] ?? 0) + 1; return a; }, {});
	const cost: Record<string, number> = {};
	for (const g of items) if (typeof g.cost === 'number') cost[String(g.currency ?? '?')] = round((cost[String(g.currency ?? '?')] ?? 0) + g.cost, 4);
	return { count: items.length, status: count((g) => g.status), mark: count((g) => g.mark), approved: items.filter((g) => g.approved === true).length, cost };
}

/** the fields record_generation takes (server RECORD_FIELDS minus toon / chapter / shot, which the tool names) */
export interface RecordArgs {
	toon: string;
	chapter: number;
	shot: string;
	file: string;
	machine: string;
	generator: string;
	kind?: 'image' | 'video';
	file_hash?: string;
	work_path?: string;
	take?: string;
	parents?: number[];
	inputs?: unknown[];
	model?: string;
	mode?: string;
	status?: 'succeeded' | 'failed' | 'cancelled';
	prompt?: string;
	seed?: number;
	ratio?: string;
	duration?: number;
	ms?: number;
	estimate?: number;
	cost?: number;
	currency?: string;
	mark?: 'review' | 'rejected';
	note?: string;
	error?: string;
	params?: Record<string, unknown>;
}

/**
 * The POST body of a record: only the fields given (the API answers 422 on an unknown one), `kind` from the file's
 * extension when left out (content's kindOf), the hash in lower case (the API takes [a-f0-9]{64}).
 */
export function recordBody(args: RecordArgs): Record<string, unknown> {
	const body: Record<string, unknown> = { ...args, kind: args.kind ?? kindOf(args.file) };
	if (typeof body.file_hash === 'string') body.file_hash = body.file_hash.toLowerCase();
	return Object.fromEntries(Object.entries(body).filter(([, v]) => v !== undefined));
}

/** the PUT body of a mark: 'none' clears it (null), a note only when given */
export function markBody(mark: 'review' | 'rejected' | 'none', note?: string): Record<string, unknown> {
	return { mark: mark === 'none' ? null : mark, ...(note !== undefined ? { note } : {}) };
}

/** the PUT body of an approval (the flag + an optional note; `approved: false` takes it back) */
export function approveBody(approved: boolean, note?: string): Record<string, unknown> {
	return { approved, ...(note !== undefined ? { note } : {}) };
}

// ── paid provider jobs (#121: server Domain\Storyboard\GenerationJobs, docs/API.md "Storyboard" → "Provider jobs") ─────

/** a generator as GET /storyboard/generators lists it */
export interface GeneratorInfo {
	id: string;
	label: string;
	configured: boolean;
	setting: string;
	unavailable: string | null;
	kinds: string[];
	models: { id: string; label: string; kind: string; mode: string; price: Record<string, unknown> | null; [field: string]: unknown }[];
}

/** the generation budget in euro (server GenerationBudget) */
export interface BudgetState {
	currency: 'EUR';
	budget: number;
	spent: number;
	reserved: number;
	left: number;
	rate: { eur_per_usd: number; date: string; source: string };
}

const num = (v: unknown): number | null => (typeof v === 'number' && Number.isFinite(v) ? v : null);

/** a model's price in words, in the provider's currency (+ euro at the server's rate) */
export function priceText(price: Record<string, unknown> | null, eurPerUsd: number | null = null): string {
	if (!price) return 'no price (not startable)';
	const cur = String(price.currency ?? 'USD');
	const eur = (usd: number) => (eurPerUsd && cur === 'USD' ? ` (≈ ${(usd * eurPerUsd).toFixed(3)} €)` : '');
	const text = num(price.text);
	const one = num(price.one_image);
	const more = num(price.per_more_image);
	if (text !== null && one !== null) return `${text} ${cur}${eur(text)} per image from text, ${one} ${cur} with 1 input image${more ? `, + ${more} per more` : ''}`;
	const perSecond = num(price.credits_per_second);
	const credit = num(price.credit) ?? 0.01;
	if (perSecond !== null) {
		const min = num(price.min_credits);
		return `${perSecond} credits/s = ${round(perSecond * credit, 3)} ${cur}/s${eur(perSecond * credit)}${min ? `, at least ${min} credits` : ''}`;
	}
	return JSON.stringify(price);
}

/** list_generators: what can run here, the models with their prices in words, the budget */
export function generatorSummary(generators: GeneratorInfo[], budget: BudgetState) {
	return {
		generators: generators.map((g) =>
			compact({
				id: g.id,
				label: g.label,
				startable: g.configured && !g.unavailable,
				why_not: g.unavailable ?? (g.configured ? undefined : `no key on the server (the owner sets ${g.setting})`),
				models: g.models.map((m) => compact({ id: m.id, label: m.label, kind: m.kind, mode: m.mode, refs: m.refs, ratios: m.ratios, durations: m.durations, price: priceText(m.price, budget.rate.eur_per_usd) }))
			})
		),
		budget: budgetText(budget)
	};
}

/** the budget in one line */
export function budgetText(b: BudgetState): string {
	if (b.budget <= 0) return 'No generation budget is set – paid generations are off until the owner sets one (admin → Settings).';
	return `Budget ${b.budget} € · spent ${b.spent.toFixed(2)} € · running ${b.reserved.toFixed(2)} € · left ${b.left.toFixed(2)} €`;
}

/** a reference as start_generation takes it: a local file (uploaded to the work storage first), a work file, a public URL */
export interface RefArg {
	role: 'still' | 'control' | 'keyframe' | 'reference' | 'character' | 'style';
	file?: string;
	work?: string;
	url?: string;
}

export const WORK_EXT: Record<string, string> = { png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', webp: 'image/webp', mp4: 'video/mp4', webm: 'video/webm' };

/** the work-storage name a local reference is uploaded under: by its content, so the same file is stored once */
export function refUploadName(shot: string, sha256: string, file: string): string {
	const ext = (/\.([a-z0-9]+)$/i.exec(file)?.[1] ?? '').toLowerCase();
	if (!WORK_EXT[ext]) throw new Error(`${file}: a reference is a png, jpg, webp, mp4 or webm`);
	return `${shot.replace(/[^\w.-]/g, '-')}-ref-${sha256.slice(0, 12)}.${ext === 'jpeg' ? 'jpg' : ext}`;
}

/** the POST body of a provider job: only the fields given */
export function jobBody(args: { toon: string; chapter: number; shot: string; generator: string; model: string; prompt?: string; aspect?: string; params?: Record<string, unknown>; take?: string; parents?: number[]; note?: string }, references: { role: string; work?: string; url?: string }[]) {
	const body: Record<string, unknown> = { ...args, ...(references.length ? { references } : {}) };
	return Object.fromEntries(Object.entries(body).filter(([, v]) => v !== undefined && v !== ''));
}

/** the file extension of a result: the content type, else the link's path */
export function resultExt(contentType: string | null, url: string | null = null): string {
	const type = (contentType ?? '').split(';')[0].trim().toLowerCase();
	const byType: Record<string, string> = { 'image/png': 'png', 'image/jpeg': 'jpg', 'image/webp': 'webp', 'video/mp4': 'mp4', 'video/webm': 'webm', 'video/quicktime': 'mov' };
	if (byType[type]) return byType[type];
	const m = url ? /\.(png|jpe?g|webp|mp4|webm|mov)(?:\?|$)/i.exec(url) : null;
	return m ? m[1].toLowerCase().replace('jpeg', 'jpg') : 'bin';
}

/** this computer's label for `machine` (SCUTOONS_MACHINE, else the host name) in the API's form */
export function machineLabel(env: NodeJS.ProcessEnv, hostname: string): string {
	const raw = (env.SCUTOONS_MACHINE || hostname || 'machine').replace(/[^A-Za-z0-9 ._-]/g, '-').replace(/^[^A-Za-z0-9]+/, '');
	return (raw || 'machine').slice(0, 64);
}

/** where a result goes in the member's workspace folder: <workspace>/<toon>/<chapter>/ */
export const resultDir = (workspace: string, toon: string, chapter: number): string[] => [workspace, toon, String(chapter)];
