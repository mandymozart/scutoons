// The generation record (from scripts/storyboard/generated-log.mjs, #119; research/VARIANTS-CANVAS.md §3.7,
// GENERATORS.md §3): one record per generation – what made each file. Online it is a row of `storyboard_generations`,
// the budget ledger (STUDIO-IN-CREATOR §2.2); here the record format and its pure logic: ids, the shot, the kind,
// parents and take from the inputs, the mark lines, the log fold, the next free name. No file system.

export const LOG = 'generated.jsonl';
export const MARKS = ['review', 'rejected'] as const;
export type GenerationMark = (typeof MARKS)[number];
export const SEED_MAX = 2 ** 32 - 1;
const VIDEO = /\.(mp4|webm|mov|m4v)$/i;
const MARK_KEYS = ['v', 'id', 'mark', 'marked'];
const PASS = '(?:grey|beauty|depth|normals)';
const TAKE_VIDEO = new RegExp(`^out:[a-z0-9-]+/handheld/[\\w-]+?--([\\w-]+?)--${PASS}\\.\\w+$`);
const TAKE_FRAME = new RegExp(`^out:[a-z0-9-]+/handheld/[\\w-]+?--([\\w-]+?)/${PASS}/f\\d+\\.png$`);
const GENERATED_KEY = /^out:[a-z0-9-]+\/generated\/([^/]+)\.[a-z0-9]+$/i;

export type InputRole = 'still' | 'control' | 'keyframe' | 'reference' | 'character' | 'style';

/** an input of a generation: its role and key ('out:<ch>/…' | 'boards:…' | 'media:…', or null + name) */
export interface GenerationInput {
	role: InputRole | string;
	key: string | null;
	name?: string;
	/** a frame of a video input */
	frame?: 'first' | 'last' | number;
	at?: number;
}

export interface Money {
	credits?: number | null;
	usd?: number | null;
	eur?: number | null;
}

/** a v2 generation record (every field present, null when unknown) */
export interface GenerationRecord {
	v: 2;
	/** <shot>--g<n> */
	id: string;
	shot: string | null;
	/** <id>.<ext>; null for a failed / cancelled job */
	file: string | null;
	kind: 'image' | 'video';
	/** the control video's or the handheld frame's take */
	take: string | null;
	generator: string | null;
	model: string | null;
	mode: string | null;
	/** the provider's task id */
	job: string | null;
	status: 'succeeded' | 'failed' | 'cancelled' | string;
	/** ids of every generated input, the main input first */
	parents: string[];
	inputs: GenerationInput[];
	prompt: string | null;
	seed: number | null;
	ratio: string | null;
	duration: number | null;
	estimate: Money | null;
	cost: Money | null;
	date: string | null;
	mark: GenerationMark | null;
	marked: string | null;
	/** a failed job's message */
	error?: string;
	/** the generator's own settings (comfy's LoRA …) */
	params?: Record<string, unknown>;
	/** approve.mjs: the approved still (the online record carries it) */
	approved?: boolean | string | null;
}

/** a mark line: the last per id wins */
export interface MarkLine {
	v: 2;
	id: string;
	mark: GenerationMark | null;
	marked: string | null;
}

export const kindOf = (file: unknown): 'image' | 'video' => (VIDEO.test(String(file)) ? 'video' : 'image');
/** a file name → its record id (the name without its extension) */
export const idOf = (file: unknown): string => String(file).replace(/\.[^.]+$/, '');
/** the shot of an id or a generated file name (<shot>--…) */
export const shotOf = (name: unknown): string | null => /^([\w-]+?)--/.exec(String(name))?.[1] ?? null;
/** a seed for a model that takes one (Runway: 0 … 2³² − 1); random = a [0, 1) source */
export const pickSeed = (random: () => number = Math.random): number => Math.floor(random() * (SEED_MAX + 1));
/** an input key without a frame marker: "frame:out:…/x.mp4#first" → "out:…/x.mp4" */
const bare = (key: unknown) => String(key ?? '').replace(/^frame:/, '').replace(/#.*$/, '');

const slashes = (p: string) => String(p).replaceAll('\\', '/').replace(/\/+$/, '');
/**
 * a file as an input key ("out:<ch>/…" before "media:…" before "boards:…" – _out lies inside storyboards), or null.
 * Paths are compared as strings (absolute, either separator); the old tool resolved them with node:path first.
 */
export function keyOf(roots: Partial<Record<'out' | 'media' | 'boards', string>> | null | undefined, file: string): string | null {
	const abs = slashes(file);
	for (const name of ['out', 'media', 'boards'] as const) {
		const base = roots?.[name] && slashes(roots[name]!);
		if (base && abs.startsWith(base + '/')) return `${name}:${abs.slice(base.length + 1)}`;
	}
	return null;
}

/** the ids of the generated inputs, in the inputs' order (the main input first), each once; parentOf(key) → the id
 *  of the generated file an input is a copy of (the approved image), or null */
export function parentsOf(inputs: { key?: string | null }[] | null | undefined, parentOf: ((key: string) => string | null) | null = null): string[] {
	const ids: string[] = [];
	for (const i of inputs ?? []) {
		const id = GENERATED_KEY.exec(bare(i.key))?.[1] ?? parentOf?.(bare(i.key)) ?? null;
		if (id && !ids.includes(id)) ids.push(id);
	}
	return ids;
}

/** the take a record belongs to: its control video's or handheld frame's take, null for anything else */
export function takeOf(inputs: { key?: string | null }[] | null | undefined): string | null {
	for (const i of inputs ?? []) {
		const k = bare(i.key);
		const m = TAKE_VIDEO.exec(k) ?? TAKE_FRAME.exec(k);
		if (m) return m[1]!;
	}
	return null;
}

export type RecordFields = Partial<Omit<GenerationRecord, 'v' | 'inputs' | 'mark' | 'marked'>> & {
	id: string;
	inputs?: (Omit<GenerationInput, 'key'> & { key?: string | null })[];
};

/** a v2 record, every field present (null when unknown); kind, take and parents derived unless given; date = now unless given */
export function record(f: RecordFields, now: () => string = () => new Date().toISOString()): GenerationRecord {
	const inputs = (f.inputs ?? []).map(({ role, key, ...rest }) => ({ role, key: key ?? null, ...rest }));
	return {
		v: 2, id: f.id, shot: f.shot ?? shotOf(f.id), file: f.file ?? null,
		kind: f.kind ?? (f.file ? kindOf(f.file) : 'image'), take: f.take !== undefined ? f.take : takeOf(inputs),
		generator: f.generator ?? null, model: f.model ?? null, mode: f.mode ?? null, job: f.job ?? null, status: f.status ?? 'succeeded',
		parents: f.parents ?? parentsOf(inputs), inputs, prompt: f.prompt ?? null, seed: f.seed ?? null, ratio: f.ratio ?? null,
		duration: f.duration ?? null, estimate: f.estimate ?? null, cost: f.cost ?? null, date: f.date !== undefined ? f.date : now(),
		mark: null, marked: null,
		...(f.error ? { error: String(f.error) } : {}),
		...(f.params && Object.values(f.params).some((x) => x != null) ? { params: f.params } : {})
	};
}

export const markLine = (id: string, mark?: GenerationMark | null, marked?: string | null): MarkLine => ({ v: 2, id, mark: mark ?? null, marked: marked ?? null });
export const isMarkLine = (j: object): boolean => Object.keys(j).every((k) => MARK_KEYS.includes(k));

export interface MarkState {
	mark: GenerationMark | null;
	marked: string | null;
	/** the mark before the last one (what "X again" brings back) */
	before: { mark: GenerationMark | null; marked: string | null } | null;
}

/** a log's text → its lines parsed: [{ raw, json }] (json null for a broken line) */
export function parseLines(text: string): { raw: string; json: unknown }[] {
	return text.split('\n').filter((l) => l.trim()).map((raw) => {
		try { return { raw, json: JSON.parse(raw) as unknown }; } catch { return { raw, json: null }; }
	});
}

/** the log folded: records (v2, log order, the last mark line per id applied), the marks of every id (also files
 *  without a record), the old lines (no v, never read as records), the broken lines' count */
export function foldLog(lines: { json: unknown }[]): { records: Map<string, GenerationRecord & MarkState>; marks: Map<string, MarkState>; earlier: unknown[]; broken: number } {
	const records = new Map<string, GenerationRecord & MarkState>(), marks = new Map<string, MarkState>(), earlier: unknown[] = [];
	let broken = 0;
	for (const { json } of lines) {
		if (!json || typeof json !== 'object') { broken++; continue; }
		const j = json as Partial<GenerationRecord> & Record<string, unknown>;
		if (j.v !== 2) { earlier.push(j); continue; }
		if (typeof j.id !== 'string' || !j.id) { broken++; continue; }
		const cur = marks.get(j.id);
		if (isMarkLine(j)) {
			marks.set(j.id, { mark: j.mark ?? null, marked: j.marked ?? null, before: cur ? { mark: cur.mark, marked: cur.marked } : null });
			continue;
		}
		records.set(j.id, { ...records.get(j.id), ...j } as GenerationRecord & MarkState);
		marks.set(j.id, { mark: j.mark ?? null, marked: j.marked ?? null, before: null });
	}
	for (const [id, r] of records) Object.assign(r, marks.get(id));
	return { records, marks, earlier, broken };
}

/** the next free name of a shot's result: the highest g<n> + 1 over the names taken (files AND the log's ids / files –
 *  a failed job's id, a deleted file's record) – never reuses an id. ext null → the id only */
export function nextName(shot: string, taken: string[], ext: string | null = null, after = 0): { id: string; file: string | null; n: number } {
	const re = new RegExp(`^${shot.replace(/[^\w]/g, '\\$&')}--g(\\d+)(?:\\.|$)`);
	const n = Math.max(after, ...taken.map((f) => Number(re.exec(f)?.[1] ?? 0))) + 1;
	const id = `${shot}--g${n}`;
	return { id, file: ext ? `${id}.${ext}` : null, n };
}

/** the names a log takes for nextName: every line's id and file name */
export const takenByLog = (lines: { json: unknown }[]): string[] =>
	lines.flatMap(({ json }) => {
		if (!json || typeof json !== 'object') return [];
		const j = json as { id?: unknown; file?: unknown };
		return [String(j.id ?? ''), String(j.file ?? '').split(/[\\/]/).pop() ?? ''];
	});
