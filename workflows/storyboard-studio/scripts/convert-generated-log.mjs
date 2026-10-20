// The generated log → v2, once (agents/research/VARIANTS-CANVAS.md §3.7 "The existing log"; RULES "Stored format changes"
// in spirit – a local log: a script with a report, never a silent load-time conversion). Every old line of
// storyboards/_out/<ch>/generated.jsonl (comfy.mjs, gemini.mjs, the studio's jobs before 2026-10-20) becomes a v2 record
// (scripts/storyboard/generated-log.mjs) in its place:
//   - the input basenames resolved inside the chapter: _out/<ch>/clean · frames · generated · variants · handheld,
//     storyboards/dataset/files/<ch>/<shot>/ (the approved image), storyboards/style/ (+ refs/), media/characters/;
//     comfy.mjs / gemini.mjs always sent _out/<ch>/clean/<shot>.png as image 1 (their code) – a name found once is
//     resolved, a name found twice or nowhere is NOT (never guessed): kept as { role, key: null, name } and reported;
//   - the roles from the generator's mode: comfy image 1 = the still (edited), luma / gemini image 1 = the layout
//     reference, then style (storyboards/style/) or character; runway image to video: the still; video to video: the
//     control video first, the still its keyframe at 0;
//   - parents where an input is a generated file, the take from a handheld control video.
// A line it can't convert stays as it is (the studio lists its file under "earlier – no record"): no shot / file, a file
// that isn't <shot>--g<n>.<ext>, an earlier line for a file a later line names again (the old count + 1 namers
// overwrote it), a file written more than 5 minutes after its line (overwritten by a drop), an unknown generator.
// Dry run by default (the report only); --write rewrites the log – the old one kept as generated.jsonl.v1 (a later run
// that still finds old lines: .v1-<ms>) – and a second run finds nothing to do. Run by hand, never on load:
//
//   node workflows/storyboard-studio/scripts/convert-generated-log.mjs [--ch s1c3,s1c4] [--write] [--json]
//        [--boards <storyboards folder>] [--media <media root>]

import { existsSync, readFileSync, readdirSync, renameSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { basename, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { LOG, idOf, record } from '../../../scripts/storyboard/generated-log.mjs';
import { ROOT, mediaRoot } from '../../../scripts/storyboard/lib.mjs';

const MEDIA = /\.(png|jpe?g|webp|mp4|webm|mov|m4v)$/i;
const VIDEO = /\.(mp4|webm|mov|m4v)$/i;
const LATE_MS = 5 * 60 * 1000;
// the fields a studio job's old line had (jobs.mjs before v2) – anything else was its generator's `meta`
const JOB_KEYS = new Set(['date', 'shot', 'file', 'engine', 'generator', 'model', 'job', 'mode', 'duration', 'ratio', 'estimate', 'cost', 'images', 'prompt']);

const ls = (dir) => { try { return readdirSync(dir, { withFileTypes: true }); } catch { return []; } };
function walk(dir, deep, found = []) {
	for (const e of ls(dir)) {
		const f = resolve(dir, e.name);
		if (e.isDirectory()) { if (deep) walk(f, deep, found); } else if (MEDIA.test(e.name)) found.push(f);
	}
	return found;
}

/** basename → [keys] of everything a chapter's generations could have used (the dataset's per shot) */
function indexOf({ boards, media, ch }) {
	const out = resolve(boards, '_out');
	const index = new Map();
	const add = (root, base, file) => {
		const key = `${root}:${relative(base, file).replaceAll('\\', '/')}`;
		const name = basename(file);
		if (!index.has(name)) index.set(name, []);
		if (!index.get(name).includes(key)) index.get(name).push(key);
	};
	for (const dir of ['clean', 'frames', 'generated', 'variants', 'handheld']) for (const f of walk(resolve(out, ch, dir), false)) add('out', out, f);
	for (const f of walk(resolve(boards, 'style'), true)) add('boards', boards, f);
	if (media) for (const f of walk(resolve(media, 'characters'), true)) add('media', media, f);
	const styleTop = new Set(walk(resolve(boards, 'style'), false).map((f) => basename(f)));
	const dataset = (shot) => new Map(walk(resolve(boards, 'dataset', 'files', ch, shot), false).map((f) => [basename(f), `boards:${relative(boards, f).replaceAll('\\', '/')}`]));
	return { index, styleTop, dataset };
}

/** which generator wrote an old line, and how its images map to roles */
function sourceOf(j) {
	if (j.engine === 'comfy' || (!j.engine && !j.generator && (j.lora || j.steps)))
		return { generator: 'comfy-local', model: 'qwen-image-edit', mode: 'comfy', first: 'still', clean: true, params: { unet: j.model, lora: j.lora, steps: j.steps, cfg: j.cfg, refMethod: j.refMethod, charLora: j.charLora, look: j.look, host: j.host } };
	if (j.engine === 'luma') return { generator: 'luma', model: j.model ?? null, mode: 'image', first: 'reference', clean: true, ratio: j.aspect, params: { style: j.style } };
	if (!j.engine && !j.generator && /^gemini/i.test(j.model ?? '')) return { generator: 'gemini', model: j.model, mode: 'image', first: 'reference', clean: true };
	if (j.generator) {
		const params = Object.fromEntries(Object.entries(j).filter(([k]) => !JOB_KEYS.has(k)));
		return { generator: j.generator, model: j.model ?? null, mode: j.mode ?? null, job: true, params };
	}
	return null;
}

/** one old line → { rec } or { why }; unresolved inputs reported through `miss` */
function convertLine(j, { ch, idx, miss }) {
	const shot = typeof j?.shot === 'string' ? j.shot : null;
	if (!shot || !/^[\w-]+$/.test(shot)) return { why: 'no shot' };
	if (typeof j.file !== 'string' || !new RegExp(`^${shot}--g\\d+\\.\\w+$`).test(j.file)) return { why: `the file ${j.file ?? '(none)'} is not ${shot}--g<n>.<ext>` };
	const src = sourceOf(j);
	if (!src) return { why: 'unknown generator' };
	const dataset = idx.dataset(shot);
	const resolveName = (name, role) => {
		const keys = [...(idx.index.get(name) ?? []), ...(dataset.has(name) ? [dataset.get(name)] : [])];
		if (keys.length === 1) return keys[0];
		miss({ name, role, why: keys.length ? `ambiguous (${keys.join(' · ')})` : 'not found' });
		return null;
	};
	const names = (Array.isArray(j.images) ? j.images : []).map(String);
	const inputs = [];
	const add = (role, name, key, extra = {}) => inputs.push({ role, key: key ?? null, ...(key ? {} : { name }), ...extra });
	if (src.job) {
		// the studio's jobs: [still, control, …characters, …style] by name – the roles from the mode, the main input first
		const v2v = src.mode === 'video_to_video';
		const control = names.find((n) => VIDEO.test(n));
		const still = names.find((n) => !VIDEO.test(n));
		if (control) add('control', control, resolveName(control, 'control'));
		if (still) add(v2v ? 'keyframe' : 'still', still, resolveName(still, v2v ? 'keyframe' : 'still'), v2v ? { at: 0 } : {});
		for (const n of names.filter((x) => x !== control && x !== still)) {
			const role = idx.styleTop.has(n) ? 'style' : 'character';
			add(role, n, resolveName(n, role));
		}
	} else {
		// comfy.mjs / gemini.mjs: image 1 = _out/<ch>/clean/<shot>.png (their code), then style or character
		names.forEach((n, i) => {
			if (i === 0) return add(src.first, n, src.clean && n === `${shot}.png` ? `out:${ch}/clean/${n}` : resolveName(n, src.first));
			const role = idx.styleTop.has(n) ? 'style' : 'character';
			add(role, n, resolveName(n, role));
		});
	}
	const params = src.params && Object.fromEntries(Object.entries(src.params).filter(([, x]) => x != null));
	return {
		rec: record({
			id: idOf(j.file), shot, file: j.file, generator: src.generator, model: src.model, mode: src.mode, job: j.job ?? null, status: 'succeeded',
			inputs, prompt: typeof j.prompt === 'string' ? j.prompt : null, seed: Number.isInteger(j.seed) ? j.seed : null, ratio: j.ratio ?? src.ratio ?? null,
			duration: j.duration ?? null, estimate: j.estimate ?? null, cost: j.cost ?? null, date: j.date ?? null, params
		})
	};
}

/** one chapter's log: the report, and with write the log rewritten (v2 lines in the old lines' places) */
export function convertLog({ boards, media = null, ch, write = false }) {
	const out = resolve(boards, '_out');
	const file = resolve(out, ch, LOG);
	const report = { ch, lines: 0, v2: 0, converted: 0, kept: [], unresolved: [], broken: 0, written: false };
	if (!existsSync(file)) return report;
	const before = readFileSync(file, 'utf8');
	const raws = before.split('\n').filter((l) => l.trim());
	const parsed = raws.map((raw) => { try { return JSON.parse(raw); } catch { return null; } });
	report.lines = raws.length;
	const idx = indexOf({ boards, media, ch });
	const gen = resolve(out, ch, 'generated');
	// the last old line per file is the one its file belongs to; an id a v2 record has is taken
	const lastOld = new Map(), v2ids = new Set();
	parsed.forEach((j, i) => { if (j?.v === 2) v2ids.add(j.id); else if (j && typeof j.file === 'string') lastOld.set(j.file, i); });
	const lines = raws.map((raw, i) => {
		const j = parsed[i];
		if (!j || typeof j !== 'object') { report.broken++; return raw; }
		if (j.v === 2) { report.v2++; return raw; }
		const keep = (why) => { report.kept.push({ line: i + 1, file: j.file ?? null, why }); return raw; };
		if (typeof j.file === 'string' && lastOld.get(j.file) !== i) return keep(`overwritten – line ${lastOld.get(j.file) + 1} names the same file`);
		if (typeof j.file === 'string' && v2ids.has(idOf(j.file))) return keep('a v2 record has this id already');
		const st = typeof j.file === 'string' && existsSync(resolve(gen, j.file)) ? statSync(resolve(gen, j.file)) : null;
		if (st && Date.parse(j.date) && st.mtimeMs > Date.parse(j.date) + LATE_MS) return keep(`the file is newer than its line (${new Date(st.mtimeMs).toISOString()}) – written over later`);
		const res = convertLine(j, { ch, idx, miss: (m) => report.unresolved.push({ line: i + 1, file: j.file, ...m }) });
		if (res.why) return keep(res.why);
		report.converted++;
		return JSON.stringify(res.rec);
	});
	if (write && report.converted) {
		// lines appended while this ran (comfy.mjs, the studio) are kept: they come after what was read
		const now = readFileSync(file, 'utf8');
		if (!now.startsWith(before)) throw new Error(`${ch}: the log changed while converting – run again`);
		writeFileSync(`${file}.tmp`, lines.join('\n') + '\n' + now.slice(before.length).replace(/^\n+/, ''));
		// one more look right before the swap: a line appended meanwhile is never lost
		if (readFileSync(file, 'utf8') !== now) { rmSync(`${file}.tmp`); throw new Error(`${ch}: the log changed while converting – run again`); }
		// the log as it was, kept: generated.jsonl.v1 – a later run that still finds old lines keeps its own copy
		const backup = existsSync(`${file}.v1`) ? `${file}.v1-${Date.now()}` : `${file}.v1`;
		writeFileSync(backup, now);
		renameSync(`${file}.tmp`, file);
		report.written = true;
		report.backup = basename(backup);
	}
	return report;
}

/** the chapters with a log under storyboards/_out */
export const chaptersOf = (boards) => ls(resolve(boards, '_out')).filter((e) => e.isDirectory() && existsSync(resolve(boards, '_out', e.name, LOG))).map((e) => e.name).sort();

export function printReport(r, write) {
	const lines = [`${r.ch}: ${r.lines} lines – ${r.converted} old → v2, ${r.v2} v2 already, ${r.kept.length} kept as they are${r.broken ? `, ${r.broken} broken` : ''}`];
	if (r.kept.length) lines.push('  kept (their files: "earlier – no record"):', ...r.kept.map((k) => `    line ${k.line} ${k.file ?? ''} – ${k.why}`));
	if (r.unresolved.length) lines.push('  inputs not resolved (kept as { role, key: null, name }):', ...r.unresolved.map((u) => `    line ${u.line} ${u.file} – ${u.role} ${u.name}: ${u.why}`));
	lines.push(r.written ? `  written (the old log: ${r.backup})` : r.converted ? `  dry run – nothing written${write ? '' : ' (--write applies it)'}` : '  nothing to convert');
	return lines.join('\n');
}

const cli = process.argv[1] && resolve(process.argv[1]).toLowerCase() === fileURLToPath(import.meta.url).toLowerCase();
if (cli) {
	const args = process.argv.slice(2);
	const take = (f) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : null; };
	const flag = (f) => { const i = args.indexOf(f); return i >= 0 ? (args.splice(i, 1), true) : false; };
	const boards = resolve(take('--boards') ?? resolve(ROOT, 'storyboards'));
	const media = take('--media') ?? mediaRoot();
	const only = take('--ch')?.split(',');
	const write = flag('--write'), json = flag('--json');
	if (only?.some((c) => !/^[a-z0-9-]+$/.test(c))) { console.error(`bad --ch ${only.join(',')}`); process.exit(2); }
	if (args.length) { console.error(`unknown ${args.join(' ')} – usage: convert-generated-log.mjs [--ch s1c3,…] [--write] [--json] [--boards dir] [--media dir]`); process.exit(2); }
	const reports = (only ?? chaptersOf(boards)).map((ch) => convertLog({ boards, media, ch, write }));
	console.log(json ? JSON.stringify(reports, null, 2) : reports.length ? reports.map((r) => printReport(r, write)).join('\n') : `no ${LOG} under ${resolve(boards, '_out')}`);
}
