// The generated log (agents/research/VARIANTS-CANVAS.md §3.7, GENERATORS.md §3): storyboards/_out/<ch>/generated.jsonl,
// one JSON line per generation – what made each file under _out/<ch>/generated/<shot>--g<n>.<ext>. Append-only, v2:
//   { v: 2, id: '<shot>--g<n>', shot, file: '<id>.<ext>' | null (a failed / cancelled job), kind: 'image' | 'video',
//     take: '<take id>' | null (the control video's or the handheld frame's take), generator, model, mode,
//     job (the provider's task id), status: 'succeeded' | 'failed' | 'cancelled',
//     parents: [ids – every input under generated/ (a frame counts as its video, the approved
//              image as the file approved – approve.mjs `from`), the main input first],
//     inputs: [{ role: 'still' | 'control' | 'keyframe' | 'reference' | 'character' | 'style', key: 'out:<ch>/…' |
//              'boards:…' | 'media:…' (generators/index.mjs) | null + name (the conversion couldn't resolve it),
//              frame?: 'first' | 'last' | <ms>, at? }],
//     prompt, seed, ratio, duration, estimate: { credits, usd } | null, cost: { credits, usd } | null, date,
//     mark: null, marked: null, error? (a failed job's message), params? (the generator's own settings: comfy's LoRA …) }
//   a mark line { v: 2, id, mark: 'review' | 'rejected' | null, marked: '<date-time>' | null } – the last per id wins
//   (approve appends mark: null; chosen is no mark – approve.mjs's `from` in the dataset manifest).
// Old lines (no `v`) are never read as a record (the old studio's one-off converter went with it, #128).
// Writers: comfy.mjs, gemini.mjs (the creator keeps its own record: @scutoons/content storyboard/generations.ts). Plain
// Node, no dependencies.

import { randomInt } from 'node:crypto';
import { appendFileSync, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { basename, relative, resolve, sep } from 'node:path';

export const LOG = 'generated.jsonl';
export const MARKS = ['review', 'rejected'];
export const SEED_MAX = 2 ** 32 - 1;
const VIDEO = /\.(mp4|webm|mov|m4v)$/i;
const MARK_KEYS = ['v', 'id', 'mark', 'marked'];
const PASS = '(?:grey|beauty|depth|normals)';
const TAKE_VIDEO = new RegExp(`^out:[a-z0-9-]+/handheld/[\\w-]+?--([\\w-]+?)--${PASS}\\.\\w+$`);
const TAKE_FRAME = new RegExp(`^out:[a-z0-9-]+/handheld/[\\w-]+?--([\\w-]+?)/${PASS}/f\\d+\\.png$`);
const GENERATED_KEY = /^out:[a-z0-9-]+\/generated\/([^/]+)\.[a-z0-9]+$/i;

export const logFile = (out, ch) => resolve(out, ch, LOG);
export const kindOf = (file) => (VIDEO.test(String(file)) ? 'video' : 'image');
/** a file name → its record id (the name without its extension) */
export const idOf = (file) => String(file).replace(/\.[^.]+$/, '');
/** the shot of an id or a generated file name (<shot>--…) */
export const shotOf = (name) => /^([\w-]+?)--/.exec(String(name))?.[1] ?? null;
/** a seed for a model that takes one (Runway: 0 … 2³² − 1) when none was given */
export const pickSeed = () => randomInt(0, SEED_MAX + 1);
/** an input key without a frame marker: "frame:out:…/x.mp4#first" → "out:…/x.mp4" */
const bare = (key) => String(key ?? '').replace(/^frame:/, '').replace(/#.*$/, '');

/** a file as an input key ("out:<ch>/…" before "media:…" before "boards:…" – _out lies inside storyboards), or null */
export function keyOf(roots, file) {
	const abs = resolve(file);
	for (const name of ['out', 'media', 'boards']) {
		const base = roots?.[name] && resolve(roots[name]);
		if (base && abs.startsWith(base + sep)) return `${name}:${relative(base, abs).replaceAll('\\', '/')}`;
	}
	return null;
}
/** the ids of the generated inputs, in the inputs' order (the main input first), each once; parentOf(key) → the id of
 *  the generated file an input is a copy of (the approved image – generators/index.mjs), or null */
export function parentsOf(inputs, parentOf = null) {
	const ids = [];
	for (const i of inputs ?? []) {
		const id = GENERATED_KEY.exec(bare(i.key))?.[1] ?? parentOf?.(bare(i.key)) ?? null;
		if (id && !ids.includes(id)) ids.push(id);
	}
	return ids;
}
/** the take a record belongs to: its control video's or handheld frame's take, null for anything else */
export function takeOf(inputs) {
	for (const i of inputs ?? []) {
		const k = bare(i.key);
		const m = TAKE_VIDEO.exec(k) ?? TAKE_FRAME.exec(k);
		if (m) return m[1];
	}
	return null;
}

/** a v2 record, every field present (null when unknown); kind, take and parents derived unless given */
export function record(f) {
	const inputs = (f.inputs ?? []).map(({ role, key, ...rest }) => ({ role, key: key ?? null, ...rest }));
	return {
		v: 2, id: f.id, shot: f.shot ?? shotOf(f.id), file: f.file ?? null,
		kind: f.kind ?? (f.file ? kindOf(f.file) : 'image'), take: f.take !== undefined ? f.take : takeOf(inputs),
		generator: f.generator ?? null, model: f.model ?? null, mode: f.mode ?? null, job: f.job ?? null, status: f.status ?? 'succeeded',
		parents: f.parents ?? parentsOf(inputs), inputs, prompt: f.prompt ?? null, seed: f.seed ?? null, ratio: f.ratio ?? null,
		duration: f.duration ?? null, estimate: f.estimate ?? null, cost: f.cost ?? null, date: f.date !== undefined ? f.date : new Date().toISOString(),
		mark: null, marked: null,
		...(f.error ? { error: String(f.error) } : {}),
		...(f.params && Object.values(f.params).some((x) => x != null) ? { params: f.params } : {})
	};
}
export const markLine = (id, mark, marked) => ({ v: 2, id, mark: mark ?? null, marked: marked ?? null });
const isMarkLine = (j) => Object.keys(j).every((k) => MARK_KEYS.includes(k));

/** appends one line (a record or a mark line) */
export function appendLine(out, ch, line) {
	mkdirSync(resolve(out, ch), { recursive: true });
	appendFileSync(logFile(out, ch), JSON.stringify(line) + '\n');
}

/** the log's lines parsed: [{ raw, json }] (json null for a broken line); none when there is no log */
export function readLines(out, ch) {
	const f = logFile(out, ch);
	if (!existsSync(f)) return [];
	return readFileSync(f, 'utf8').split('\n').filter((l) => l.trim()).map((raw) => {
		try { return { raw, json: JSON.parse(raw) }; } catch { return { raw, json: null }; }
	});
}

/** the log read: records (v2, log order, the last mark line per id applied), the marks of every id (also files
 *  without a record) with the mark before the last one (`before` – what "X again" brings back), the old lines still
 *  there (never read as records), the broken lines' count */
export function readLog(out, ch) {
	const records = new Map(), marks = new Map(), earlier = [];
	let broken = 0;
	for (const { json: j } of readLines(out, ch)) {
		if (!j || typeof j !== 'object') { broken++; continue; }
		if (j.v !== 2) { earlier.push(j); continue; }
		if (typeof j.id !== 'string' || !j.id) { broken++; continue; }
		const cur = marks.get(j.id);
		if (isMarkLine(j)) {
			marks.set(j.id, { mark: j.mark ?? null, marked: j.marked ?? null, before: cur ? { mark: cur.mark, marked: cur.marked } : null });
			continue;
		}
		records.set(j.id, { ...records.get(j.id), ...j });
		marks.set(j.id, { mark: j.mark ?? null, marked: j.marked ?? null, before: null });
	}
	for (const [id, r] of records) Object.assign(r, marks.get(id));
	return { records, marks, earlier, broken };
}

/** the next free name of a shot's result: the highest g<n> + 1 over the files in generated/ AND the log (a failed
 *  job's id, a deleted file's record) – never overwrites a file, never reuses an id. ext null → the id only */
export function nextName(out, ch, shot, ext = null, after = 0) {
	const re = new RegExp(`^${shot.replace(/[^\w]/g, '\\$&')}--g(\\d+)(?:\\.|$)`);
	const dir = resolve(out, ch, 'generated');
	const names = existsSync(dir) ? readdirSync(dir) : [];
	for (const { json } of readLines(out, ch)) if (json) names.push(String(json.id ?? ''), basename(String(json.file ?? '')));
	const n = Math.max(after, ...names.map((f) => Number(re.exec(f)?.[1] ?? 0))) + 1;
	const id = `${shot}--g${n}`;
	return { id, file: ext ? `${id}.${ext}` : null, n };
}

/** writes a result file under the next free name, created exclusively ('wx'): a writer in another process (comfy.mjs
 *  beside the studio) that took the same name a moment earlier is never overwritten – the next n then */
export function writeNew(out, ch, shot, ext, bytes) {
	mkdirSync(resolve(out, ch, 'generated'), { recursive: true });
	for (let after = 0; ;) {
		const { id, file, n } = nextName(out, ch, shot, ext, after);
		try {
			writeFileSync(resolve(out, ch, 'generated', file), bytes, { flag: 'wx' });
			return { id, file };
		} catch (e) {
			if (e.code !== 'EEXIST') throw e;
			after = n;
		}
	}
}
