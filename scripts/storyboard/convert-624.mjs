// Converts a chapter's standard slices from 390 × 625 to the new standard 390 × 624 (#78; owner 2026-10-20: "i would go
// for it. do it now"; RULES "Product rules": its 3x 1170 × 1872 = WEBTOON's 800 × 1280) – no Figma, no bridge (owner:
// "the bridge or figma crapped out a couple of times … i dont like manually restarting"). Each standard slice is
// resampled (PIL Lanczos, 625 → 624 rows, at every density it has) from its own lossless PNG export in
// storyboards/_out/s<k>c<n>/{clean,reslice,production}/png, proven to be the right one: the sibling slices/<name>{,.2x,.3x}
// .webp are byte-identical to every file the slice shows on the CDN now (read fresh each run: the asset manager can
// replace a file in place). No proof or no PNG → the CDN's own WebPs are the source, listed as a warning. Boxes, items,
// events and areas are % of the slice and stay (≤ 1 px at 1x, research map of #78).
//
//   node scripts/storyboard/convert-624.mjs --key chapter:catpope:1 [--folder s1/c1/v3] [--sheet] [--upload] [--verify]
//
// Dry run by default: reads the newest draft (read-only MCP: read_entity, search_assets, validate) and the CDN, makes the
// files, writes nothing outside storyboards/_out/s<k>c<n>/convert-624/ – plan.json, ops.json (the save_draft it WOULD
// send, run through validate), slices/*.webp.
//   --folder   s<k>/c<n>/v<N> below the chapter's own folder: the new CDN folder; default the next free v<N> (s1/c2 has
//              v2–v4 → v5), kept from this run folder's previous plan – uploads never replace, so one new folder per chapter
//   --sheet    convert-624/sheet.html: old | new 1x of a few slices and every seam (bottom 20 px of slice i over the top
//              20 px of slice i + 1), old and new, 2× zoomed – open it locally before --upload
//   --upload   validate errors → stop; upload_asset of every new file (409 = there already: must be byte-identical, else
//              stop; uploads.json skips files confirmed before), then ONE save_draft on the base read at the start:
//              image, imageVariants, height 624 per slice – nothing else. A save in between: if the planned slices are
//              unchanged there, the same ops once more on the new base; else stop, nothing saved (a re-run plans anew)
//   --verify   (with --upload, or on its own later) the newest draft points every planned slice at its new files,
//              624 high, and every new file answers 200 on the CDN with the right size and the bytes made here
//   --workers  python processes in parallel (default min(4, cores − 1)); --cdn another CDN root
// Resumes: made.json records per slice where its outputs came from (the CDN paths, the source, the sha256 of every source
// file and output) – a slice is made again when any of it changed (re-cut art, a new export: a warning says so).
// Watches itself: every network call has a timeout and retries, the MCP server is started again once if it dies (its last
// stderr lines are shown when it can't start), a python worker without progress for STALL_MS is killed, one run per
// chapter at a time (convert-624/run.lock) – after any stop just run it again, it goes on where it was.
// A save is never sent twice. On Windows closing the MCP client ends only the launcher (bin/scutoons-mcp.mjs) – this script
// ends the server's whole tree, but a request already sent stays sent and can still land: after a save without a clear
// answer the run watches the draft for LAND_MS, and every later run (and a 409) checks whether the plan's save is in the
// draft and records it (plan.saved.recovered), so --verify works.
// One file on purpose: plan, files, upload, save and verify share the plan and its checks. Its own size reader works on
// bytes (the CDN's answers, not only files – lib.mjs imageSize reads a file) and made.json records more per slice than
// lib.mjs runManifest (the source's and the outputs' hashes).
// The Figma drafts of chapters 1–5 stay on the 625 grid: a re-cut from them goes through figma-clean-slices.mjs --grid 625.

import { execFile, spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { closeSync, existsSync, mkdirSync, openSync, readFileSync, readSync, renameSync, rmSync, writeFileSync, writeSync } from 'node:fs';
import { cpus, homedir } from 'node:os';
import { basename, dirname, relative, resolve } from 'node:path';
import { Client } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';
import { ROOT, SLICE_H, W, esc } from './lib.mjs';

const OLD_H = 625; // the standard until 2026-10-20 (chapters 1–5 were cut at it)
const SUF = { 1: '', 2: '.2x', 3: '.3x' }; // density → file suffix (ASSETS.md "Variants are suffixes")
const DENSITY_OF = { low: 1, medium: 2, high: 3 }; // Slice.imageVariants (content.ts)
const KINDS = ['clean', 'reslice', 'production']; // where figma-clean-slices / figma-reslice / figma-production export
const FOLDER = /^s\d+\/c\d+\/v\d+$/; // ASSETS.md: <toon>/s<k>/c<n>/v<N>/<name>.webp
const PATH = /^[\w-]+(\/[\w-][\w.-]*)*\.webp$/i; // a toon-relative WebP path (no segment starts with a dot: no "..")
const FETCH_MS = 30_000, STALL_MS = 180_000, LAND_MS = 120_000;

/** a stop with its exit code (2 = how it was called) – printed by the catch at the end */
class Stop extends Error {
	constructor(message, exit = 1) { super(message); this.exit = exit; }
}

const USAGE = 'usage: convert-624.mjs --key chapter:<toon>:<n> [--folder s<k>/c<n>/v<N>] [--sheet] [--upload] [--verify] [--workers 4] [--cdn https://…]';
const args = process.argv.slice(2);
const usage = (why) => { console.error(`${why}\n${USAGE}`); process.exit(2); };
const take = (f, d) => {
	const i = args.indexOf(f);
	if (i < 0) return d;
	const v = args[i + 1];
	if (v === undefined || v.startsWith('--')) usage(`${f} needs a value`); // "--folder --upload" never makes "--upload" the folder
	args.splice(i, 2);
	return v;
};
const flag = (f) => { const i = args.indexOf(f); return i >= 0 ? (args.splice(i, 1), true) : false; };
const key = take('--key'), folderArg = take('--folder'), CDN = String(take('--cdn', 'https://cdn.scutoons.com')).replace(/\/+$/, '');
const workers = Number(take('--workers', Math.min(4, Math.max(1, cpus().length - 1))));
const upload = flag('--upload'), verify = flag('--verify'), sheet = flag('--sheet');
const km = /^chapter:([\w-]+):(\d+)$/.exec(key ?? '');
if (!km) usage(`--key ${key ?? '(missing)'}: not chapter:<toon>:<n>`);
if (args.length) usage(`unknown: ${args.join(' ')}`);
if (folderArg && !FOLDER.test(folderArg)) usage(`--folder ${folderArg}: not s<k>/c<n>/v<N>`);
if (!Number.isInteger(workers) || workers < 1) usage('--workers: a whole number ≥ 1');
if (!/^https?:\/\/[^/\s]+$/.test(CDN)) usage(`--cdn ${CDN}: not an http(s) origin`);
const [, toon, num] = km;

// ---------- small helpers: hashes, pixel sizes from file headers, atomic writes, fetch with timeout + retries ----------

const sha = (buf) => createHash('sha256').update(buf).digest('hex');
const shaOf = (file) => sha(readFileSync(file));
const sleep = (ms) => new Promise((ok) => setTimeout(ok, ms));
const pause = (ms) => Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms); // a short synchronous wait
const head = (file, n = 32) => { const fd = openSync(file, 'r'); try { const b = Buffer.alloc(n); readSync(fd, b, 0, n, 0); return b; } finally { closeSync(fd); } };
/** [width, height] from a file's first 30 bytes: PNG (IHDR) or WebP (VP8 lossy, VP8L lossless, VP8X extended); else null */
function dims(b) {
	if (b.length >= 24 && b.readUInt32BE(0) === 0x89504e47) return [b.readUInt32BE(16), b.readUInt32BE(20)];
	if (b.length < 30 || b.toString('ascii', 0, 4) !== 'RIFF' || b.toString('ascii', 8, 12) !== 'WEBP') return null;
	const chunk = b.toString('ascii', 12, 16);
	if (chunk === 'VP8X') return [1 + b.readUIntLE(24, 3), 1 + b.readUIntLE(27, 3)];
	if (chunk === 'VP8 ') return [b.readUInt16LE(26) & 0x3fff, b.readUInt16LE(28) & 0x3fff];
	if (chunk === 'VP8L') { const v = b.readUInt32LE(21); return [1 + (v & 0x3fff), 1 + ((v >>> 14) & 0x3fff)]; }
	return null;
}
const sizeOf = (file) => (existsSync(file) ? dims(head(file)) : null);
const is = (d, w, h) => !!d && d[0] === w && d[1] === h;
const fmt = (d) => (d ? `${d[0]}×${d[1]}` : 'none');
/** the same entries (density → path / hash), whatever their order */
const same = (a, b) => JSON.stringify(Object.entries(a ?? {}).sort()) === JSON.stringify(Object.entries(b ?? {}).sort());
/** temp file + rename; the rename is retried briefly while Windows (a virus scanner, the indexer) holds the target */
function writeAtomic(file, data) {
	const tmp = `${file}.${process.pid}.tmp`;
	writeFileSync(tmp, data);
	for (let t = 1; ; t++) {
		try { return renameSync(tmp, file); } catch (e) { if (t >= 6 || !['EPERM', 'EBUSY', 'EACCES'].includes(e.code)) throw e; pause(250 * t); }
	}
}
const readJson = (file, d) => { try { return JSON.parse(readFileSync(file, 'utf8')); } catch { return d; } };
const rel = (f) => relative(ROOT, f).replaceAll('\\', '/');

/** GET with a timeout, 3 tries; { status, body } – status 404 as an answer, 0 = no answer (error) */
async function get(url, tries = 3) {
	for (let t = 1; ; t++) {
		try {
			const r = await fetch(url, { signal: AbortSignal.timeout(FETCH_MS) });
			if (r.status === 404) return { status: 404 };
			if (!r.ok) throw new Error(`HTTP ${r.status}`);
			return { status: r.status, body: Buffer.from(await r.arrayBuffer()) };
		} catch (e) {
			if (t >= tries) return { status: 0, error: e.name === 'TimeoutError' ? `timeout ${FETCH_MS / 1000} s` : e.message };
			await sleep(2000 * t);
		}
	}
}
/** fn over items, n at a time */
async function pool(items, n, fn) {
	let next = 0;
	await Promise.all(Array.from({ length: Math.min(n, items.length) }, async () => { while (next < items.length) { const i = next++; await fn(items[i], i); } }));
}

/** one run per chapter at a time (two would share plan.json, made.json and the outputs' temp files) → release() */
function lockRun(file) {
	const alive = (pid) => { try { process.kill(pid, 0); return true; } catch (e) { return e.code === 'EPERM'; } };
	for (;;) {
		try {
			const fd = openSync(file, 'wx');
			try { writeSync(fd, JSON.stringify({ pid: process.pid, at: new Date().toISOString(), args: process.argv.slice(2) })); } finally { closeSync(fd); }
			return () => { if (readJson(file, {}).pid === process.pid) rmSync(file, { force: true }); };
		} catch (e) {
			if (e.code !== 'EEXIST') throw e;
			const o = readJson(file, {});
			if (o.pid && o.pid !== process.pid && alive(o.pid))
				throw new Stop(`another convert-624 run works in ${rel(dirname(file))} (pid ${o.pid} since ${o.at}: ${(o.args ?? []).join(' ')}) – wait for it; delete run.lock only if that process is gone`);
			rmSync(file, { force: true }); // left by a run that died
		}
	}
}

// ---------- the SCUTOONS MCP over stdio (~/.claude.json's "scutoons" entry: the member's own rights) ----------

const cfg = readJson(resolve(homedir(), '.claude.json'), {}).mcpServers?.scutoons;
if (!cfg) { console.error('no "scutoons" MCP entry in ~/.claude.json'); process.exit(2); }
let client, transport, errTail = [];
const tail = () => (errTail.length ? `\n  the MCP server's last words:\n${errTail.map((l) => `    ${l}`).join('\n')}` : '');
const connect = async () => {
	client = new Client({ name: 'scutoons-convert-624', version: '1.0.0' });
	transport = new StdioClientTransport({ command: cfg.command, args: cfg.args, env: { ...process.env, ...cfg.env }, stderr: 'pipe' });
	// its stderr: the last 20 lines, shown when it can't start or dies (always read – a full pipe would block the server)
	errTail = [];
	transport.stderr?.on('data', (d) => { errTail.push(...String(d).split(/\r?\n/).filter((l) => l.trim())); errTail.splice(0, Math.max(0, errTail.length - 20)); });
	try {
		await client.connect(transport);
	} catch (e) {
		await sleep(300);
		throw new Error(`the SCUTOONS MCP server did not start: ${e.message}${tail()}`);
	}
};
/** ends our server: on Windows close() ends only the launcher, so its whole tree goes (pid is null once the launcher is gone) */
const disconnect = async () => {
	const pid = transport?.pid;
	if (pid && process.platform === 'win32') await new Promise((ok) => execFile('taskkill', ['/pid', String(pid), '/T', '/F'], { windowsHide: true }, () => ok()));
	await client?.close().catch(() => {});
};
// one tool call; no answer (the server died, a timeout) → the server is started again once and the call repeated –
// never for save_draft (it must not run twice: the caller looks what happened)
const call = async (name, a, timeout = 120_000, retry = name !== 'save_draft') => {
	try {
		const r = await client.callTool({ name, arguments: a }, { timeout });
		const text = (r.content ?? []).map((c) => c.text ?? '').join('\n');
		let data; try { data = JSON.parse(text); } catch { data = text; }
		return { error: !!r.isError, data };
	} catch (e) {
		if (!retry) throw new Error(`${name}: ${e.message}${tail()}`);
		console.log(`\n${name}: ${e.message} – starting the MCP server again (once)${tail()}`);
		await disconnect();
		await connect();
		return call(name, a, timeout, false);
	}
};
const said = (d) => (typeof d === 'string' ? d : JSON.stringify(d) ?? '').slice(0, 400);
/** a 409 as the server words it (apiMessage: "409 conflict: …") – not any "409" inside a list of problems */
const conflict = (r) => r.error && typeof r.data === 'string' && /^409\b/.test(r.data);
const readDraft = async () => {
	const r = await call('read_entity', { key });
	if (r.error || !r.data?.content || !r.data?.version) throw new Error(`read_entity ${key}: ${said(r.data)}`);
	return r.data;
};
/** how many files the asset manager has below <toon>/<folder>/ (search_assets matches the path) */
const filesIn = async (folder) => {
	const r = await call('search_assets', { q: `${toon}/${folder}/`, page_size: 100 }, 60_000);
	if (r.error) throw new Error(`search_assets: ${said(r.data)}`);
	return (r.data?.items ?? []).filter((a) => String(a.path ?? '').startsWith(`${toon}/${folder}/`)).length;
};

/** how many of a plan's slices the draft already shows at their new files, 624 high (its save landed) */
function landed(plan, doc) {
	const all = (doc.groups ?? []).flatMap((g) => g.slices ?? []);
	return plan.slices.filter((p) => {
		const here = doc.groups?.[p.group]?.slices?.[p.index];
		return (here?.image === p.to.image ? here : all.find((s) => s.image === p.to.image))?.height === SLICE_H;
	}).length;
}
/** every planned slice still sits where the plan found it, as it was (390 × 625, the same files) */
function unchanged(plan, doc) {
	return plan.slices.every((p) => {
		const s = doc.groups?.[p.group]?.slices?.[p.index];
		return !!s && !s.video && s.image === p.from.image && s.width === W && s.height === OLD_H && same(s.imageVariants, p.from.variants);
	});
}

// ---------- python workers: resample + WebP q90 method 6 (like figma-production / figma-clean-slices), atomic ----------

const PY = `import sys, json, os, time
from PIL import Image
for j in json.load(open(sys.argv[1], encoding='utf-8')):
    im = Image.open(j['src']).convert('RGB')
    if im.size != (j['sw'], j['sh']):
        print(json.dumps({'bad': j['dst'], 'size': list(im.size)}), flush=True)
        continue
    tmp = '%s.%d.tmp' % (j['dst'], os.getpid())
    im.resize((j['w'], j['h']), Image.Resampling.LANCZOS).save(tmp, 'WEBP', quality=90, method=6)
    for t in range(6):
        try:
            os.replace(tmp, j['dst'])
            break
        except PermissionError:
            if t == 5: raise
            time.sleep(0.25 * (t + 1))
    print(json.dumps({'ok': j['dst']}), flush=True)`;

/** one python process over a job file; killed when it prints nothing for STALL_MS (a hung process never blocks the run) */
function runWorker(file, onLine) {
	return new Promise((ok, fail) => {
		const child = spawn(process.env.PYTHON ?? 'python', ['-I', '-c', PY, file], { stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true });
		let last = Date.now(), err = '', buf = '';
		const dog = setInterval(() => {
			if (Date.now() - last < STALL_MS) return;
			clearInterval(dog); child.kill();
			fail(new Error(`python: no progress for ${STALL_MS / 1000} s – killed (run again: it resumes)`));
		}, 5000);
		child.stdout.on('data', (d) => {
			last = Date.now(); buf += d;
			for (let i; (i = buf.indexOf('\n')) >= 0; buf = buf.slice(i + 1)) { const line = buf.slice(0, i).trim(); if (line) onLine(JSON.parse(line)); }
		});
		child.stderr.on('data', (d) => { err += d; });
		child.on('error', (e) => { clearInterval(dog); fail(e); });
		child.on('close', (code) => { clearInterval(dog); code === 0 ? ok() : fail(new Error(`python exited ${code}: ${err.trim().split('\n').slice(-3).join(' | ')}`)); });
	});
}

// ---------- the run ----------

let release = () => {};

async function main() {
	const cur = await readDraft();
	const doc = cur.content, groups = doc.groups ?? [];
	let base = cur.version.id;
	// the run folder beside the Figma scripts' exports: storyboards/_out/s<k>c<n>/ (the season from the chapter's own paths)
	const seasons = {};
	for (const g of groups) for (const s of g.slices ?? []) { const x = new RegExp(`^(s\\d+)/c${num}/`).exec(s.image ?? ''); if (x) seasons[x[1]] = (seasons[x[1]] ?? 0) + 1; }
	const season = Object.entries(seasons).sort((a, b) => b[1] - a[1])[0]?.[0] ?? 's1';
	const OUT = resolve(ROOT, 'storyboards/_out', `${season}c${num}`), RUN = resolve(OUT, 'convert-624');
	for (const d of ['slices', 'cdn']) mkdirSync(resolve(RUN, d), { recursive: true });
	release = lockRun(resolve(RUN, 'run.lock'));
	const planFile = resolve(RUN, 'plan.json'), opsFile = resolve(RUN, 'ops.json'), prev = readJson(planFile, null);
	console.log(`${key} · newest draft v${base} (${cur.version.status}${cur.version.published ? `, published v${cur.version.published}` : ''}) · ${rel(RUN)}`);
	// the previous plan's save in the draft though this script never heard of it (no answer, a crash, a late save) → recorded
	if (prev?.key === key && prev.slices?.length && !prev.saved && landed(prev, doc) === prev.slices.length) {
		prev.saved = { version: base, recovered: true, at: new Date().toISOString() };
		writeAtomic(planFile, JSON.stringify(prev, null, '\t'));
		console.log(`the previous plan's save is in the draft (found in v${base}) – recorded in plan.json`);
	}
	if (verify && !upload) return verifyRun(prev, RUN);

	// 1. the standard slices of the neutral chapter (everything else stays as it is)
	const warnings = [], picks = [], perGroup = [], names = new Map();
	for (const [gi, g] of groups.entries()) {
		const label = g.label ?? `#${gi}`;
		if (g.ref) throw new Error(`group ${gi} "${label}" is a stored scene (${g.ref.src}) – its slices live in their own key (not built: catpope has none)`);
		const row = { group: gi, label, slices: (g.slices ?? []).length, convert: 0, kept: {} };
		perGroup.push(row);
		for (const [si, s] of (g.slices ?? []).entries()) {
			const where = `${label}[${si}] ${s.image}`, keep = () => { const k = `${s.width}×${s.height}`; row.kept[k] = (row.kept[k] ?? 0) + 1; };
			if (s.width !== W || s.height !== OLD_H) { keep(); continue; }
			if (s.video) { warnings.push(`${where}: has a video – left at ${OLD_H} (a video is not resampled)`); keep(); continue; }
			if (!PATH.test(s.image ?? '')) { warnings.push(`${where}: not a WebP in the toon's folder – left at ${OLD_H}`); keep(); continue; }
			const name = basename(s.image).replace(/\.webp$/i, '');
			const dens = { 1: s.image }, vmap = {}; // density → its CDN path now; variant key → density
			let odd = '';
			for (const [k, v] of Object.entries(s.imageVariants ?? {})) {
				const d = typeof v !== 'string' || !PATH.test(v) ? 0 : v === s.image ? 1 : /\.2x\.webp$/i.test(v) ? 2 : /\.3x\.webp$/i.test(v) ? 3 : DENSITY_OF[k];
				if (!d || (dens[d] && dens[d] !== v)) { odd = `imageVariants.${k} = ${v}`; break; }
				dens[d] = v; vmap[k] = d;
			}
			if (odd) { warnings.push(`${where}: ${odd} – no clear density, left at ${OLD_H}`); keep(); continue; }
			if (names.has(name) && names.get(name) !== s.image) throw new Error(`two slices share the name ${name} (${names.get(name)}, ${s.image}) – the new folder can't hold both`);
			names.set(name, s.image);
			row.convert++;
			picks.push({ group: gi, label, index: si, name, dens, vmap, variants: s.imageVariants });
		}
	}
	if (!picks.length) {
		console.log(`nothing to convert: no ${W} × ${OLD_H} slice in v${base}${prev?.saved ? ` (plan.json of the save v${prev.saved.version} kept – --verify checks it)` : ''}`);
		if (verify && prev?.saved) await verifyRun(prev, RUN);
		return;
	}

	// 2. the source of each slice, proven: a local export whose WebPs are byte-identical to every file the slice shows now
	const seen = {};
	let fetched = 0;
	try {
		await pool(picks, 8, async (p) => {
			const want = Object.keys(p.dens).map(Number);
			p.cdn = {};
			for (const d of want) {
				const r = await get(`${CDN}/${toon}/${p.dens[d]}`);
				if (r.status === 404) { p.missing = p.dens[d]; return; }
				if (!r.body) throw new Error(`CDN ${toon}/${p.dens[d]}: ${r.error} – nothing written, run again`);
				p.cdn[d] = { body: r.body, sha256: sha(r.body) };
				seen[`${toon}/${p.dens[d]}`] = { sha256: p.cdn[d].sha256, bytes: r.body.length, size: dims(r.body) };
				process.stdout.write(`\rCDN ${++fetched} files read   `);
			}
			const why = [];
			for (const kind of KINDS) {
				const dir = resolve(OUT, kind), webp = (d) => resolve(dir, 'slices', `${p.name}${SUF[d]}.webp`), png = (d) => resolve(dir, 'png', `${p.name}@${d}x.png`);
				if (!existsSync(webp(1)) || shaOf(webp(1)) !== p.cdn[1].sha256) continue;
				const differ = want.filter((d) => d !== 1 && (!existsSync(webp(d)) || shaOf(webp(d)) !== p.cdn[d].sha256));
				if (differ.length) { why.push(`${kind}: the CDN's ${differ.map((d) => p.dens[d]).join(', ')} not byte-identical to its slices/${p.name}${differ.map((d) => SUF[d]).join('|')}.webp`); continue; }
				const bad = want.filter((d) => !is(sizeOf(png(d)), W * d, OLD_H * d));
				if (bad.length) { why.push(`${kind}/png: ${bad.map((d) => `@${d}x`).join(' ')} missing or not ${W} × ${OLD_H} × density`); continue; }
				p.source = 'png'; p.kind = kind;
				p.src = Object.fromEntries(want.map((d) => [d, png(d)]));
				p.srcSha = Object.fromEntries(want.map((d) => [d, shaOf(png(d))]));
				delete p.cdn;
				return;
			}
			p.source = 'cdn'; p.why = why.join('; ') || `no local export is byte-identical to ${p.dens[1]}`;
		});
	} finally {
		writeAtomic(resolve(RUN, 'cdn-hashes.json'), JSON.stringify(seen, null, '\t')); // what the CDN showed (a record, not a cache)
		if (fetched) console.log();
	}
	for (const p of picks.filter((x) => x.missing)) { warnings.push(`${p.label}[${p.index}] ${p.missing}: 404 on the CDN – left at ${OLD_H}`); delete p.cdn; }
	const todo = picks.filter((p) => !p.missing);

	// the CDN's own WebPs where nothing local is proven: the bytes just read, kept as convert-624/cdn/<toon>/<CDN path>
	for (const p of todo.filter((x) => x.source === 'cdn')) {
		const off = Object.entries(p.cdn).filter(([d, c]) => !is(dims(c.body), W * d, OLD_H * d));
		if (off.length) p.skip = off.map(([d, c]) => `${p.dens[d]} is ${fmt(dims(c.body))}, not ${W * d}×${OLD_H * d}`).join('; ');
		else {
			p.src = {}; p.srcSha = {};
			for (const [d, c] of Object.entries(p.cdn)) {
				const file = resolve(RUN, 'cdn', toon, p.dens[d]);
				mkdirSync(dirname(file), { recursive: true });
				if (!existsSync(file) || shaOf(file) !== c.sha256) writeAtomic(file, c.body);
				p.src[d] = file; p.srcSha[d] = c.sha256;
			}
		}
		delete p.cdn;
		warnings.push(p.skip ? `${p.label}[${p.index}] ${p.dens[1]}: ${p.skip} – left at ${OLD_H}` : `${p.label}[${p.index}] ${p.name}: source = the CDN WebPs (lossy) – ${p.why}`);
	}
	const slices = todo.filter((p) => !p.skip);
	if (!slices.length) {
		console.log(`warnings (${warnings.length}):\n${warnings.map((w) => `  - ${w}`).join('\n')}`);
		throw new Stop(`none of the ${picks.length} standard slices can be converted (the warnings above) – nothing planned; is ${CDN} the CDN?`);
	}

	// 3. resample: 625 → 624 rows at every density; made.json says per slice where its outputs came from (the CDN paths, the
	//    source, the sha256 of every source file and output) – resumed only when all of it is the same now, else made again
	const madeFile = resolve(RUN, 'made.json'), made = readJson(madeFile, {});
	const out = (p, d) => resolve(RUN, 'slices', `${p.name}${SUF[d]}.webp`);
	const stale = (p) => { // null = its outputs are current; 'new' / 'old' (no record of their source) / why they are made again
		const m = made[p.name], ds = Object.keys(p.dens);
		if (!ds.every((d) => existsSync(out(p, d)))) return 'new';
		if (!m?.from || !m.src || !m.out) return 'old';
		if (!same(m.from, p.dens)) return `the slice shows other files now (${m.from[1]} → ${p.dens[1]})`;
		if (m.source !== p.source || m.kind !== p.kind) return `its source is ${p.kind ? `${p.kind}/png` : 'the CDN'} now (was ${m.kind ? `${m.kind}/png` : 'the CDN'})`;
		if (!same(m.src, p.srcSha)) return `its source files changed (${p.kind ? `${p.kind}/png` : 'the CDN WebPs'})`;
		if (!ds.every((d) => is(sizeOf(out(p, d)), W * d, SLICE_H * d) && m.out[d] === shaOf(out(p, d)))) return 'its outputs are not the ones recorded';
		return null;
	};
	const jobs = [];
	let old = 0;
	for (const p of slices) {
		const w = stale(p);
		if (w === null) continue;
		if (w === 'old') old++;
		else if (w !== 'new') warnings.push(`${p.label}[${p.index}] ${p.name}: made again – ${w}`);
		delete made[p.name]; // a crash before its files are all there leaves no record → made again
		jobs.push(p);
	}
	writeAtomic(madeFile, JSON.stringify(made, null, '\t'));
	const madeNow = jobs.reduce((a, p) => a + Object.keys(p.dens).length, 0);
	if (jobs.length) {
		const n = Math.min(workers, jobs.length), bad = [], owner = new Map(), left = new Map(); // output → slice; slice → files to come
		for (const p of jobs) { left.set(p, Object.keys(p.dens).length); for (const d of Object.keys(p.dens)) owner.set(out(p, d), p); }
		let done = 0;
		const parts = Array.from({ length: n }, (_, w) => jobs.filter((_, i) => i % n === w));
		await Promise.all(parts.map(async (part, w) => {
			const file = resolve(RUN, `jobs-${w}.json`);
			writeFileSync(file, JSON.stringify(part.flatMap((p) => Object.keys(p.dens).map((d) => ({ src: p.src[d], dst: out(p, d), sw: W * d, sh: OLD_H * d, w: W * d, h: SLICE_H * d })))));
			try {
				await runWorker(file, (m) => {
					if (m.bad) { bad.push(`${basename(m.bad)}: source ${fmt(m.size)}`); return; }
					process.stdout.write(`\rresample ${++done}/${madeNow} files (${n} python)   `);
					const p = owner.get(m.ok);
					left.set(p, left.get(p) - 1);
					if (left.get(p)) return;
					// all its files are there: where they came from, and what they are
					made[p.name] = { source: p.source, ...(p.kind ? { kind: p.kind } : {}), from: p.dens, src: p.srcSha, out: Object.fromEntries(Object.keys(p.dens).map((d) => [d, shaOf(out(p, d))])), at: new Date().toISOString() };
					writeAtomic(madeFile, JSON.stringify(made, null, '\t'));
				});
			} finally {
				rmSync(file, { force: true });
			}
		}));
		console.log();
		if (bad.length) throw new Error(`sources with the wrong size: ${bad.join(', ')}`);
	}
	// every output's size, read from the file (also the resumed ones)
	const wrong = slices.flatMap((p) => Object.keys(p.dens).filter((d) => !is(sizeOf(out(p, d)), W * d, SLICE_H * d)).map((d) => `${p.name}${SUF[d]}: ${fmt(sizeOf(out(p, d)))}`));
	if (wrong.length) throw new Error(`outputs with the wrong size: ${wrong.join(', ')}`);

	// 4. the new CDN folder: --folder, else the previous plan's, else the next free v<N> beside the chapter's own
	const counts = {};
	for (const p of slices) { const b = /^(s\d+\/c\d+)\//.exec(p.dens[1])?.[1] ?? `${season}/c${num}`; counts[b] = (counts[b] ?? 0) + 1; }
	const baseDir = Object.entries(counts).sort((a, b) => b[1] - a[1])[0][0];
	let folder = folderArg, folderNote = 'given';
	if (!folder && prev?.key === key && prev.folder) { folder = prev.folder; folderNote = 'from the previous plan'; }
	if (folder && (!FOLDER.test(folder) || !folder.startsWith(`${baseDir}/`)))
		throw new Stop(`folder ${folder} (${folderNote}) is not s<k>/c<n>/v<N> below the chapter's ${baseDir}/ – ${folderNote === 'given' ? 'check --folder' : `pass --folder or delete ${rel(planFile)}`}`, 2);
	if (!folder) {
		const used = new Set(), walk = (v) => {
			if (typeof v === 'string') { const x = new RegExp(`^${baseDir}/v(\\d+)/`).exec(v); if (x) used.add(Number(x[1])); }
			else if (v && typeof v === 'object') for (const c of Object.values(v)) walk(c);
		};
		walk(doc);
		let v = Math.max(1, ...used) + 1;
		while (await filesIn(`${baseDir}/v${v}`)) v++;
		folder = `${baseDir}/v${v}`;
		folderNote = `next free – the chapter uses ${used.size ? [...used].sort((a, b) => a - b).map((x) => `v${x}`).join(', ') : `no v<N> under ${baseDir}`}`;
	} else {
		const there = await filesIn(folder);
		if (there) folderNote += `, ${there} files there already (a 409 must be byte-identical)`;
	}

	// 5. the plan, the save_draft it would send, the creator's checks on the draft with these ops (validate, read-only)
	const to = (p, d) => `${folder}/${p.name}${SUF[d]}.webp`;
	const plan = {
		key, base, folder, made: new Date().toISOString(),
		slices: slices.map((p) => ({
			group: p.group, label: p.label, index: p.index, name: p.name,
			from: { image: p.dens[1], variants: p.variants ?? null },
			to: { image: to(p, 1), variants: p.variants ? Object.fromEntries(Object.entries(p.vmap).map(([k, d]) => [k, to(p, d)])) : null },
			source: p.source, ...(p.kind ? { local: `${rel(OUT)}/${p.kind}` } : {}), ...(p.source === 'cdn' ? { why: p.why } : {}),
			sizes: Object.fromEntries(Object.keys(p.dens).map((d) => [`${d}x`, sizeOf(out(p, d))]))
		})),
		warnings
	};
	const ops = plan.slices.flatMap((p) => {
		const at = ['groups', p.group, 'slices', p.index];
		return [
			{ op: 'set', path: [...at, 'image'], value: p.to.image },
			...(p.to.variants ? [{ op: 'set', path: [...at, 'imageVariants'], value: p.to.variants }] : []),
			{ op: 'set', path: [...at, 'height'], value: SLICE_H }
		];
	});
	const cdnCount = plan.slices.filter((p) => p.source === 'cdn').length;
	const message = `#78: ${plan.slices.length} standard slices ${W} × ${OLD_H} → ${W} × ${SLICE_H} (resampled from the lossless exports${cdnCount ? `, ${cdnCount} from the CDN WebPs` : ''}, ${folder})`;
	const v = await call('validate', { key, ops });
	plan.validate = v.error ? { failed: said(v.data) } : { errors: v.data?.errors ?? 0, warnings: v.data?.warnings ?? 0 };
	const errs = v.error ? [] : (v.data?.problems ?? []).filter((x) => x.level === 'error').slice(0, 5).map((x) => `${(x.path ?? []).join('.')}: ${x.message}`);
	const savePlan = () => {
		writeAtomic(planFile, JSON.stringify(plan, null, '\t'));
		writeAtomic(opsFile, JSON.stringify({ key, base: plan.base, ops, message }, null, '\t'));
	};
	if (prev?.saved && prev.key === key) writeAtomic(resolve(RUN, `plan-saved-v${prev.saved.version}.json`), JSON.stringify(prev, null, '\t'));
	savePlan();
	if (sheet) writeSheet(plan, RUN);

	// the summary
	const files = plan.slices.reduce((a, p) => a + Object.keys(p.sizes).length, 0);
	const byKind = {};
	for (const p of plan.slices) if (p.local) byKind[p.local.split('/').pop()] = (byKind[p.local.split('/').pop()] ?? 0) + 1;
	console.log(`new folder ${toon}/${folder} (${folderNote})`);
	for (const g of perGroup.filter((x) => x.slices)) console.log(`  ${g.label}: ${g.convert} of ${g.slices} → ${W} × ${SLICE_H}${Object.keys(g.kept).length ? `, kept ${Object.entries(g.kept).map(([s, n]) => `${n} × ${s}`).join(', ')}` : ''}`);
	console.log(`sources: ${plan.slices.length - cdnCount} lossless PNG (${Object.entries(byKind).map(([k, n]) => `${k} ${n}`).join(', ') || '–'}; their WebPs = the CDN files now, every density), ${cdnCount} CDN WebP`);
	console.log(`files: ${files} (${madeNow} made now${old ? ` – ${old} slices again: an older run left no record of their source` : ''}, ${files - madeNow} from an earlier run) – every size checked (1x ${W}×${SLICE_H}, 2x ${W * 2}×${SLICE_H * 2}, 3x ${W * 3}×${SLICE_H * 3})`);
	console.log(plan.validate.failed ? `validate FAILED: ${plan.validate.failed}` : `validate (the draft with these ops): ${plan.validate.errors} errors, ${plan.validate.warnings} warnings${errs.length ? `\n${errs.map((x) => `  - ${x}`).join('\n')}` : ''}`);
	console.log(warnings.length ? `warnings (${warnings.length}):\n${warnings.map((w) => `  - ${w}`).join('\n')}` : 'warnings: none');
	console.log(`→ ${rel(planFile)}, ops.json (${ops.length} ops)${sheet ? ', sheet.html' : ''}`);
	if (!upload) { console.log(`dry run – nothing uploaded or saved. Next: --sheet to look, then --upload --verify`); return; }
	if (plan.validate.failed || plan.validate.errors) throw new Stop(`validate: ${plan.validate.failed ?? `${plan.validate.errors} errors`} – nothing uploaded or saved`);

	// 6. upload every new file (never replaces; uploads.json = the files confirmed on the CDN before), then ONE save_draft
	const upFile = resolve(RUN, 'uploads.json'), confirmed = readJson(upFile, {}); // CDN path → sha256 of the bytes there
	const uploads = plan.slices.flatMap((p) => Object.keys(p.sizes).map((dx) => { const d = Number(dx.slice(0, -1)); return { local: out(p, d), path: `${toon}/${to(p, d)}`, sha: shaOf(out(p, d)) }; }));
	let fresh = 0, there = 0, before = 0;
	const clash = [];
	for (const f of uploads) {
		if (confirmed[f.path] === f.sha) { before++; continue; }
		for (let t = 1; ; t++) {
			const r = await call('upload_asset', { file: f.local, path: f.path }, 180_000);
			if (!r.error) { fresh++; confirmed[f.path] = f.sha; break; }
			if (conflict(r)) { // there already: ours from an earlier run (a hang after the store, a stop), or someone else's
				const g = await get(`${CDN}/${f.path}`);
				if (g.body && sha(g.body) === f.sha) { there++; confirmed[f.path] = f.sha; }
				else clash.push(`${f.path} (${g.body ? 'other bytes' : g.error ?? `HTTP ${g.status}`})`);
				break;
			}
			if (t >= 3) throw new Error(`upload ${f.path}: ${said(r.data)} – nothing saved; run again (uploads so far are reused)`);
			await sleep(3000 * t);
		}
		writeAtomic(upFile, JSON.stringify(confirmed, null, '\t'));
		process.stdout.write(`\rupload ${fresh + there + before + clash.length}/${uploads.length} (new ${fresh}, already there ${there}, confirmed before ${before}${clash.length ? `, CLASH ${clash.length}` : ''})   `);
	}
	console.log();
	if (clash.length) throw new Error(`${clash.length} paths in ${folder} hold other files: ${clash.slice(0, 5).join(', ')}${clash.length > 5 ? ' …' : ''} – nothing saved; pick another --folder`);

	/** no clear answer: the draft is read every 10 s for LAND_MS – a save that still lands is found (never sent twice) */
	const watch = async () => {
		const t0 = Date.now();
		for (;;) {
			const now = await readDraft();
			if (now.version.id !== base || Date.now() - t0 >= LAND_MS) { console.log(); return now; }
			process.stdout.write(`\rwatching the draft: still v${base} after ${Math.round((Date.now() - t0) / 1000)} s   `);
			await sleep(10_000);
		}
	};
	const ours = (now) => String(now.version.message ?? '').startsWith(message) || landed(plan, now.content) === plan.slices.length;
	let saved = null, again = true;
	while (!saved) {
		let r;
		try { r = await call('save_draft', { key, base, ops, message }, 300_000); } catch (e) { r = { lost: e.message.replace(/^save_draft: /, '') }; }
		if (!r.lost && !r.error) { saved = { version: r.data?.version ?? null, answer: r.data }; break; }
		const unclear = !!r.lost || /^(network error|5\d\d)\b/.test(String(r.data)); // a 502 can come after the commit
		if (!unclear && !conflict(r)) throw new Stop(`save_draft refused: ${said(r.data)} – nothing saved`); // the creator's checks, 401, 403 …
		const what = r.lost ?? said(r.data);
		if (unclear) console.log(`\nsave_draft: ${what} – no clear answer: watching the draft for ${LAND_MS / 1000} s`);
		const now = unclear ? await watch() : await readDraft();
		if (now.version.id !== base && ours(now)) { saved = { version: now.version.id, recovered: true }; break; }
		if (now.version.id === base) throw new Stop(`save_draft: ${what} – v${base} is still the newest${unclear ? ` after ${LAND_MS / 1000} s` : ''}: nothing saved. Run again (a save that lands later is found and recorded then).`);
		const who = `v${base} → v${now.version.id}, ${now.version.author}: "${now.version.message}"`;
		if (again && unchanged(plan, now.content)) { // only other parts changed: the same ops on the new base, once
			again = false;
			console.log(`save_draft: ${key} was saved meanwhile (${who}) – the planned slices are unchanged there: the same ops on v${now.version.id}`);
			base = plan.base = now.version.id;
			savePlan();
			continue;
		}
		throw new Stop(`conflict: ${key} was saved meanwhile (${who})${again ? ' and the planned slices changed there' : ', again'} – nothing saved. Run again: it plans on the new draft and reuses the files and uploads.`);
	}
	const a = saved.answer ?? {};
	console.log(`save_draft: v${saved.version} saved (${ops.length} ops)${saved.recovered ? ' – this plan\'s save, found in the draft' :` · ${a.errors ?? 0} errors, ${a.warnings ?? 0} warnings`}${a.open ? ` · ${a.open}` : ''}`);
	plan.saved = { version: saved.version, ...(saved.recovered ? { recovered: true } : {}), uploads: { new: fresh, there, before }, at: new Date().toISOString() };
	savePlan();
	if (verify) await verifyRun(plan, RUN);
	else console.log('the owner checks and publishes the draft in the creator; --verify checks the draft and the CDN files');
}

/** 7. the newest draft points every planned slice at its new files (624 high), every new file is on the CDN as made here */
async function verifyRun(plan, RUN) {
	if (!plan?.slices?.length || plan.key !== key) throw new Stop(`no plan for ${key} in ${rel(RUN)} – run without --verify first`);
	// never GET the new paths before they are uploaded: the CDN could cache the 404s
	if (!plan.saved) throw new Stop(`plan.json records no save – --verify checks after --upload (it never probes paths not uploaded yet)`);
	const now = await readDraft(), groups = now.content.groups ?? [];
	const all = groups.flatMap((g, gi) => (g.slices ?? []).map((s, si) => ({ s, gi, si })));
	const bad = [];
	let ok = 0;
	for (const p of plan.slices) {
		const here = groups[p.group]?.slices?.[p.index];
		const s = here?.image === p.to.image ? here : all.find((x) => x.s.image === p.to.image)?.s;
		if (!s) { bad.push(`${p.label}[${p.index}] ${p.name}: no slice shows ${p.to.image}`); continue; }
		const errs = [];
		if (s.width !== W || s.height !== SLICE_H) errs.push(`${s.width}×${s.height}`);
		const v = s.imageVariants ?? null, want = p.to.variants;
		if (!want !== !v || (want && !same(want, v))) errs.push(`imageVariants ${JSON.stringify(v)}`);
		if (errs.length) bad.push(`${p.label}[${p.index}] ${p.name}: ${errs.join(', ')}`); else ok++;
	}
	const files = plan.slices.flatMap((p) => Object.keys(p.sizes).map((dx) => { const d = Number(dx.slice(0, -1)); return { path: `${toon}/${d === 1 ? p.to.image : p.to.image.replace(/\.webp$/i, `${SUF[d]}.webp`)}`, w: W * d, h: SLICE_H * d, local: resolve(RUN, 'slices', `${p.name}${SUF[d]}.webp`) }; }));
	let good = 0;
	await pool(files, 8, async (f) => {
		const g = await get(`${CDN}/${f.path}`);
		if (!g.body) bad.push(`${f.path}: ${g.error ?? `HTTP ${g.status}`}`);
		else if (!is(dims(g.body), f.w, f.h)) bad.push(`${f.path}: ${fmt(dims(g.body))}, not ${f.w}×${f.h}`);
		else if (existsSync(f.local) && sha(g.body) !== shaOf(f.local)) bad.push(`${f.path}: other bytes than ${basename(f.local)}`);
		else good++;
		process.stdout.write(`\rCDN ${good + bad.length}/${files.length}   `);
	});
	console.log();
	const left = all.filter((x) => x.s.width === W && x.s.height === OLD_H).length;
	console.log(`verify v${now.version.id}: ${ok}/${plan.slices.length} slices point at ${plan.folder} with ${W} × ${SLICE_H}; ${good}/${files.length} files on the CDN at the right size; ${left} slices still ${W} × ${OLD_H}`);
	if (bad.length) { console.log(`FAILED (${bad.length}):\n${bad.slice(0, 40).map((b) => `  - ${b}`).join('\n')}`); process.exitCode = 1; }
}

/** convert-624/sheet.html: old | new 1x of a few slices + every seam old and new (2× zoom), to eyeball before --upload */
function writeSheet(plan, RUN) {
	const Z = 2, n = plan.slices.length, k = Math.min(6, n);
	const picks = [...new Set(Array.from({ length: k }, (_, i) => Math.round((i * (n - 1)) / Math.max(1, k - 1))))].map((i) => plan.slices[i]);
	const oldSrc = (p) => `${CDN}/${toon}/${p.from.image}`, newSrc = (p) => `slices/${p.name}.webp`;
	const cut = (src, h, bottom) => `<div class="cut"><img src="${esc(src)}" loading="lazy" alt="" style="margin-top:${bottom ? -(h - 20) * Z : 0}px"></div>`;
	const seams = plan.slices.slice(0, -1).map((p, i) => [p, plan.slices[i + 1]]).filter(([a, b]) => a.group === b.group && b.index === a.index + 1);
	const html = `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Slices 624 check</title><style>
:root { color-scheme: light dark; --bg: #f4f4f2; --fg: #1b1b1b; --muted: #6b6b6b; --line: #d0d0cc; }
@media (prefers-color-scheme: dark) { :root { --bg: #161616; --fg: #ececec; --muted: #9a9a9a; --line: #333; } }
body { margin: 0; padding: 1rem; background: var(--bg); color: var(--fg); font: 0.875rem/1.4 system-ui, sans-serif; }
h1 { font-size: 1.125rem; font-weight: 600; } h2 { font-size: 1rem; font-weight: 600; margin-top: 2rem; }
.muted { color: var(--muted); } .row { display: flex; gap: 1rem; flex-wrap: wrap; margin: 1rem 0; align-items: flex-start; }
figure { margin: 0; } figcaption { color: var(--muted); font-size: 0.75rem; } figure img { width: ${W}px; display: block; }
.cut { width: ${W * Z}px; height: ${20 * Z}px; overflow: hidden; } .cut img { width: ${W * Z}px; display: block; image-rendering: pixelated; }
.seam { border-top: 1px solid var(--line); padding-top: 0.5rem; } .pair { display: flex; gap: 1rem; flex-wrap: wrap; }
</style></head><body>
<h1>${esc(plan.key)} · ${W} × ${OLD_H} → ${W} × ${SLICE_H}</h1>
<p class="muted">base v${plan.base} · new folder ${esc(plan.folder)} · ${n} slices · old = the CDN now, new = convert-624/slices (not uploaded)</p>
${plan.warnings.length ? `<ul>${plan.warnings.map((w) => `<li>${esc(w)}</li>`).join('')}</ul>` : ''}
<h2>Old | new (1x)</h2>
${picks.map((p) => `<div class="row"><figure><img src="${esc(oldSrc(p))}" loading="lazy" alt=""><figcaption>old · ${esc(p.from.image)}</figcaption></figure><figure><img src="${esc(newSrc(p))}" loading="lazy" alt=""><figcaption>new · ${esc(p.to.image)} · ${p.source}</figcaption></figure></div>`).join('\n')}
<h2>Seams (bottom 20 px of slice i over the top 20 px of slice i + 1, 1x zoomed ${Z}×)</h2>
${seams.map(([a, b]) => `<div class="seam"><div class="muted">${esc(a.label)} ${a.index} | ${b.index}</div><div class="pair"><div><div class="muted">old</div>${cut(oldSrc(a), OLD_H, true)}${cut(oldSrc(b), OLD_H, false)}</div><div><div class="muted">new</div>${cut(newSrc(a), SLICE_H, true)}${cut(newSrc(b), SLICE_H, false)}</div></div></div>`).join('\n')}
</body></html>`;
	writeAtomic(resolve(RUN, 'sheet.html'), html);
}

try {
	await connect();
	await main();
} catch (e) {
	console.error(`\nstopped: ${e.message}`);
	process.exitCode = e.exit ?? 1;
} finally {
	release();
	await disconnect();
}
