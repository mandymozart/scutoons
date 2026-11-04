// check-core-only.mjs (#357; v2 #363 – owner 2026-11-03: "enforce it even for dumb agents"): the ratchet that keeps deck
// logic out of the browser's TypeScript. The rule (agents/DECISIONS.md 2026-11-02 "One universal core", RULES "Code"): all
// deck logic lives in deck-core (Rust, native + wasm); the browser only has thin adapters – bytes in and out, drawing, the
// UI's own state.
//
// What it reads – the bridge files: the bridge packages and the deck's Svelte wrappers (ROOTS), plus every .ts / .svelte
// in the apps and shared packages that imports a bridge package (or a .ts module that does – the import graph, not a path
// list). Generated files (`// GENERATED` on line 1) are their generator's.
// What it checks, per bridge file (scripts/lib/core-only-rules.mjs has the rules):
// - hits: the shapes deck logic takes (scene literals, catalog kinds, clamps, tempo maths, MIDI bytes, the core's texts,
//   the lead rule, randomness, parameter tables, mood names, Rust twins) against scripts/core-only-baseline.json's
//   `entries` – the known old cases, each with its move ticket. A hit not there, or more than it lists, fails. A known
//   false positive is an `allow` rule (file, rule, the code it matches, why). A line marked `// core-only-ok: #N <reason>`
//   (genuine adapter code) is no hit but counts in the file's `escapes` budget;
// - budgets: lines, functions, `Math.` calls and escapes per file (`budgets`); growing more than 5 % fails, a new escape
//   fails; a file without a budget gets DEFAULT_BUDGET (adapters are thin);
// - scripts/bridge-files.json: every bridge file with its one-line purpose; a new file needs a line `adapter: …` / `ui: …`;
// - data files: a scene / rig TOML or JSON under a bridge root or an app's static/ must be generated or a byte copy of
//   its deck/ source.
// The baseline only shrinks: against the one on `main` (the merge base) no entry, allow rule, budget or escape may grow
// and no rule's entry appear (a rule `main` did not know yet starts from today's count), unless a commit since then
// carries `Owner-approved-baseline: <the owner's words, date>`.
//
//   npm run check:core-only                 the check (dependency-free but for the apps' typescript; ~1 s)
//   npm run check:core-only -- --report     the baseline's size, the top hits per file, every escape
//   npm run check:core-only -- --shrink     write the baseline down to today's counts (never up)
//   npm run audit:bridge                    the check + the report + the deltas against the last `bridge-audit-N` tag
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readdirSync, readFileSync, writeFileSync, existsSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve, posix } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PATTERNS, RULE_NAMES, WHAT, emptyContext, imports, measure, rustNumbers, scan } from './lib/core-only-rules.mjs';

export { PATTERNS, scan };

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export const BASELINE = 'scripts/core-only-baseline.json';
export const BRIDGE_FILES = 'scripts/bridge-files.json';

/** where the browser's deck code lives: the bridge (live-core, live-deck, engine-host, live) and its Svelte wrappers */
export const ROOTS = [
	'shared/live-core/src',
	'shared/live-deck/src',
	'shared/engine-host/src',
	'shared/live/src',
	'viewer/src/lib/components/live-deck',
	'creator/src/lib/components/sound',
	// modules the deck pages use without importing a bridge package themselves (BRIDGE-AUDIT.md §5): the creator's signal
	// strip models the engine host's snapshot, the viewer's live-deck.ts holds the page's options and panel rules
	'creator/src/lib/editor/signal-strip.ts',
	'viewer/src/lib/live-deck.ts'
];
/** where the import graph looks for files that touch the engine */
export const SCOPE = ['viewer/src', 'creator/src', 'admin/src', 'shared', 'tools'];
/** a bridge package: importing one makes a file a bridge file (the space wasm comes through live-deck) */
export const BRIDGE_IMPORT = /^@scutoons\/(?:live|live-core|live-deck|engine-host)(?:\/|$)|space_web|live_core\.wasm/;
/** where data files (scenes, rigs) may not be hand copies */
export const DATA_ROOTS = ['shared/live-core', 'shared/live-deck', 'shared/engine-host', 'shared/live', 'viewer/static', 'creator/static'];
/** a bridge file without a budget: an adapter is thin */
export const DEFAULT_BUDGET = { lines: 150, functions: 12, math: 4, escapes: 0 };
/** how much a budgeted file may grow (lines, functions, Math. calls) before it needs the owner */
export const GROWTH = 1.05;
/** the rules and paths the first baseline (v1, #357) knew – a baseline without `rules` / `version` knows only these */
const V1_RULES = Object.keys(PATTERNS);
const V1_ROOTS = ['shared/live-core/src', 'shared/live-deck/src', 'shared/engine-host/src', 'shared/live/src', 'viewer/src/lib/components/live-deck', 'creator/src/lib/components/sound'];
const PURPOSE = /^(adapter|ui|types|logic \(#\d+(?:, #\d+)*\)): \S/;

const rel = (p, at = root) => relative(at, p).replaceAll('\\', '/');
const read = (p) => readFileSync(p, 'utf8');

function walk(dir, keep) {
	if (!existsSync(dir)) return [];
	if (statSync(dir).isFile()) return keep(dir) ? [dir] : [];
	return readdirSync(dir).flatMap((n) => {
		if (n === 'node_modules' || n === 'paraglide' || n === 'test' || n === 'tests' || n === '.svelte-kit' || n === 'dist' || n === 'build' || n === 'pkg' || n.startsWith('.')) return [];
		const p = join(dir, n);
		return statSync(p).isDirectory() ? walk(p, keep) : keep(p) ? [p] : [];
	});
}
const source = (p) => /\.(ts|svelte)$/.test(p) && !/\.(test|spec|d)\.ts$/.test(p);

/**
 * The bridge files: every source file under ROOTS, plus the files under SCOPE that import a bridge package or a .ts module
 * that is a bridge file (a .svelte importer is one, but does not make its own importers one). → [relative paths]
 */
export function bridgeFiles(at = root) {
	const rooted = new Set(ROOTS.flatMap((r) => walk(join(at, r), source)).map((p) => rel(p, at)));
	const all = [...new Set(SCOPE.flatMap((s) => walk(join(at, s), source)).map((p) => rel(p, at)))];
	const deps = new Map(all.map((f) => [f, imports(read(join(at, f)))]));
	const bridge = new Set([...rooted].filter((f) => !/^\/\/ GENERATED/.test(read(join(at, f)))));
	const resolveSpec = (from, spec) => {
		const app = from.split('/src/')[0];
		const base = spec.startsWith('$lib/') ? `${app}/src/lib/${spec.slice(5)}` : spec.startsWith('.') ? posix.join(posix.dirname(from), spec) : null;
		if (!base) return null;
		return [base, `${base}.ts`, `${base}/index.ts`].find((c) => deps.has(c)) ?? null;
	};
	let grew = true;
	while (grew) {
		grew = false;
		for (const [f, specs] of deps) {
			if (bridge.has(f)) continue;
			const touches = specs.some((s) => {
				if (BRIDGE_IMPORT.test(s)) return true;
				const t = resolveSpec(f, s);
				return t !== null && t.endsWith('.ts') && bridge.has(t) && !rooted.has(t);
			});
			if (touches && !/^\/\/ GENERATED/.test(read(join(at, f)))) {
				bridge.add(f);
				grew = true;
			}
		}
	}
	return [...bridge].sort();
}

/** the deck's vocabulary: catalog kinds (kinds.json, generated from deck/catalog), the energy ladder's moods (deck-space mood.rs, #367), Rust numbers */
export function context(at = root) {
	const ctx = emptyContext();
	const kinds = JSON.parse(read(join(at, 'server/src/Domain/Rigs/kinds.json'))).kinds;
	for (const k of kinds) ctx.kinds.add(k.name);
	const being = read(join(at, 'deck/space/src/mood.rs'));
	for (const m of /pub const MOODS: \[&str; \d+\] = \[([^\]]*)\]/.exec(being)?.[1].matchAll(/"(\w+)"/g) ?? []) ctx.moods.add(m[1]);
	// the core crates and the platform crates whose constants a browser twin would copy (not the tests, not deck/lab)
	const crates = ['core', 'engine', 'space', 'events', 'catalog', 'live-core', 'outputs', 'native', 'relay', 'link'];
	const rs = crates.flatMap((c) => walk(join(at, 'deck', c, 'src'), (p) => p.endsWith('.rs'))).map((p) => [rel(p, at), read(p)]);
	Object.assign(ctx, rustNumbers(rs));
	return ctx;
}

/** today's hits per file (after the allow rules), the escapes, the refused escapes, the measures, unused allow rules */
export function scanAll(at = root, baseline = { allow: [] }) {
	const ctx = context(at);
	const found = {};
	const hits = {};
	const excused = [];
	const bad = [];
	const sizes = {};
	const allowUsed = (baseline.allow ?? []).map(() => 0);
	const files = bridgeFiles(at);
	for (const f of files) {
		const text = read(join(at, f));
		const svelte = f.endsWith('.svelte');
		const s = scan(text, ctx, { svelte });
		const kept = s.hits.filter((h) => {
			const i = (baseline.allow ?? []).findIndex((a, k) => a.file === f && a.rule === h.rule && h.code.includes(a.match) && allowUsed[k] < (a.count ?? 1));
			if (i < 0) return true;
			allowUsed[i]++;
			return false;
		});
		const counts = {};
		for (const h of kept) counts[h.rule] = (counts[h.rule] ?? 0) + 1;
		if (Object.keys(counts).length) found[f] = counts;
		hits[f] = kept;
		excused.push(...s.excused.map((e) => ({ file: f, ...e })));
		bad.push(...s.bad.map((e) => ({ file: f, ...e })));
		sizes[f] = { ...measure(text, { svelte }), escapes: new Set(s.excused.map((e) => e.line)).size };
	}
	const unusedAllow = (baseline.allow ?? []).map((a, i) => ({ ...a, used: allowUsed[i] })).filter((a) => a.used < (a.count ?? 1));
	return { files, found, hits, excused, bad, sizes, unusedAllow };
}

/** today's hits ({file: {rule: n}}) against the baseline ({file: {rule: {count, ticket}}}) → the failures */
export function compare(found, baseline) {
	const errors = [];
	for (const [file, counts] of Object.entries(found)) {
		for (const [rule, n] of Object.entries(counts)) {
			const b = baseline[file]?.[rule];
			if (!b) errors.push(`${file}: ${n} × ${rule} – new (${WHAT[rule]})`);
			else if (n > b.count) errors.push(`${file}: ${n} × ${rule}, the baseline allows ${b.count} (${b.ticket}) – ${WHAT[rule]}`);
		}
	}
	return errors;
}

/** each file's size against its budget (none: DEFAULT_BUDGET) → the failures */
export function overBudget(sizes, budgets) {
	const errors = [];
	for (const [file, size] of Object.entries(sizes)) {
		const b = budgets[file];
		const limit = b ?? DEFAULT_BUDGET;
		for (const k of ['lines', 'functions', 'math']) {
			const max = b ? Math.floor(limit[k] * GROWTH) : limit[k];
			if (size[k] > max) errors.push(`${file}: ${size[k]} ${k}, its budget is ${limit[k]}${b ? ' (+5 %)' : ' (a new bridge file: an adapter is thin)'} – move the logic into the core, don't grow the bridge`);
		}
		if (size.escapes > limit.escapes) errors.push(`${file}: ${size.escapes} core-only-ok line(s), its budget is ${limit.escapes} – a new escape needs the owner (Owner-approved-baseline)`);
	}
	return errors;
}

/** the bridge files against scripts/bridge-files.json → the failures (a file without a purpose, a purpose without a kind) */
export function purposes(files, listed) {
	const errors = [];
	for (const f of files) {
		const p = listed[f];
		if (!p) errors.push(`${f}: a bridge file without a line in ${BRIDGE_FILES} – add "${f}": "adapter: <what it moves between the core and the platform>" (or "ui: …")`);
		else if (!PURPOSE.test(p)) errors.push(`${BRIDGE_FILES}: "${f}": "${p}" – a purpose starts with adapter: / ui: / types: / logic (#N): `);
	}
	return errors;
}

/** data files (scenes, rigs) under DATA_ROOTS: generated or a byte copy of a deck/ file → the failures */
export function dataFiles(at = root) {
	// the deck's scene and rig sources, hashed only when a bridge root holds a scene-like file
	let deckHashes = null;
	const hashes = () =>
		(deckHashes ??= new Set(
			['deck/native/scenes', 'deck/native/rigs', 'deck/conformance/fixtures', 'deck/live-core/fixtures', 'deck/outputs/fixtures']
				.flatMap((d) => walk(join(at, d), (p) => /\.(toml|json)$/.test(p)))
				.map((p) => createHash('sha256').update(readFileSync(p)).digest('hex'))
		));
	const errors = [];
	for (const p of DATA_ROOTS.flatMap((r) => walk(join(at, r), (x) => /\.(toml|json)$/.test(x) && !/package(-lock)?\.json$|tsconfig/.test(x)))) {
		const text = read(p);
		const sceneish = /^\s*\[\[node\]\]|^\s*\[rig\]/m.test(text) || /"kind"\s*:\s*"[a-z_]+"[\s\S]*"(?:links?|nodes?)"\s*:|"(?:links?|nodes?)"\s*:[\s\S]*"kind"\s*:\s*"[a-z_]+"/.test(text);
		if (!sceneish) continue;
		if (/^(?:#|\/\/) GENERATED|"\$generated"/.test(text)) continue;
		if (hashes().has(createHash('sha256').update(readFileSync(p)).digest('hex'))) continue;
		errors.push(`${rel(p, at)}: a scene / rig file that is neither generated nor a byte copy of its deck/ source – the browser loads the deck's own file (#360)`);
	}
	return errors;
}

/** the baseline against the one on `main`: it may only shrink (an approved commit may grow it) */
export function ratchet(base, current, approved) {
	if (approved) return [];
	const known = new Set(base.rules ?? V1_RULES);
	const errors = [];
	const entriesOf = (d) => d.entries ?? d;
	// a v1 baseline (no `version`) knew only v1's paths: files the import graph added start from today's count
	const scoped = (file) => base.version || V1_ROOTS.some((r) => file.startsWith(`${r}/`));
	for (const [file, rules] of Object.entries(entriesOf(current))) {
		if (!scoped(file)) continue;
		for (const [rule, { count }] of Object.entries(rules)) {
			if (!known.has(rule)) continue;
			const was = entriesOf(base)[file]?.[rule]?.count;
			if (was === undefined) errors.push(`baseline: ${file} ${rule} is new – the baseline only shrinks`);
			else if (count > was) errors.push(`baseline: ${file} ${rule} grew ${was} → ${count} – the baseline only shrinks`);
		}
	}
	if (base.allow) {
		const key = (a) => `${a.file}\u0000${a.rule}\u0000${a.match}`;
		const was = new Map(base.allow.map((a) => [key(a), a.count ?? 1]));
		for (const a of current.allow ?? []) {
			if (!was.has(key(a))) errors.push(`baseline: a new allow rule ${a.file} ${a.rule} "${a.match}" – false positives are the owner's to accept`);
			else if ((a.count ?? 1) > was.get(key(a))) errors.push(`baseline: the allow rule ${a.file} ${a.rule} "${a.match}" grew – the baseline only shrinks`);
		}
	}
	for (const [file, b] of Object.entries(current.budgets ?? {})) {
		const was = base.budgets?.[file];
		if (!base.budgets) break;
		if (!was) {
			errors.push(`baseline: a budget for ${file} is new – a new bridge file has DEFAULT_BUDGET`);
			continue;
		}
		for (const k of ['lines', 'functions', 'math', 'escapes']) if (b[k] > was[k]) errors.push(`baseline: ${file} ${k} budget grew ${was[k]} → ${b[k]} – it only shrinks`);
	}
	return errors;
}

/** the baseline written down to today's numbers (never up): entries, allow rules, budgets */
export function shrunk(found, baseline, sizes = null, unusedAllow = []) {
	const entries = {};
	for (const [file, rules] of Object.entries(baseline.entries ?? baseline)) {
		for (const [rule, b] of Object.entries(rules)) {
			const n = Math.min(b.count, found[file]?.[rule] ?? 0);
			if (n > 0) (entries[file] ??= {})[rule] = { count: n, ticket: b.ticket };
		}
	}
	if (!baseline.entries) return entries;
	const unused = new Map(unusedAllow.map((a) => [`${a.file}\u0000${a.rule}\u0000${a.match}`, a.used]));
	const allow = (baseline.allow ?? [])
		.map((a) => {
			const k = `${a.file}\u0000${a.rule}\u0000${a.match}`;
			return unused.has(k) ? { ...a, count: unused.get(k) } : a;
		})
		.filter((a) => (a.count ?? 1) > 0);
	const budgets = {};
	for (const [file, b] of Object.entries(baseline.budgets ?? {})) {
		const s = sizes?.[file];
		if (!s) continue;
		budgets[file] = Object.fromEntries(['lines', 'functions', 'math', 'escapes'].map((k) => [k, Math.min(b[k], s[k])]));
	}
	return { ...baseline, entries, allow, budgets };
}

const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();

/** the baseline on `main`'s merge base and whether a commit since then carries the owner's approval (null: none there) */
function onMain() {
	try {
		const base = git('merge-base', 'HEAD', 'main');
		const approved = /Owner-approved-baseline:\s*\S/.test(git('log', '--format=%B', `${base}..HEAD`));
		try {
			return { doc: JSON.parse(git('show', `${base}:${BASELINE}`)), approved };
		} catch {
			return { doc: null, approved };
		}
	} catch {
		return { doc: null, approved: false };
	}
}

const total = (entries) => Object.values(entries).reduce((s, p) => s + Object.values(p).reduce((a, b) => a + b.count, 0), 0);

/** `npm run audit:bridge`: the deltas against the last `bridge-audit-N` tag (the hits, the bridge's size, twin phrases in commits) */
function audit(doc, sizes) {
	let tag = '';
	try {
		tag = git('tag', '--list', 'bridge-audit-*', '--sort=-creatordate').split('\n')[0];
	} catch {
		/* no git */
	}
	const lines = Object.values(sizes).reduce((s, x) => s + x.lines, 0);
	console.log(`\naudit: ${Object.keys(sizes).length} bridge files, ${lines} lines, ${total(doc.entries)} known hits, ${(doc.allow ?? []).length} allow rules`);
	const byRule = {};
	for (const rules of Object.values(doc.entries)) for (const [r, v] of Object.entries(rules)) byRule[r] = (byRule[r] ?? 0) + v.count;
	console.log(`  per rule: ${Object.entries(byRule).sort((a, b) => b[1] - a[1]).map(([r, n]) => `${r} ${n}`).join(' · ')}`);
	const byTicket = {};
	for (const rules of Object.values(doc.entries)) for (const v of Object.values(rules)) byTicket[v.ticket] = (byTicket[v.ticket] ?? 0) + v.count;
	console.log(`  per ticket: ${Object.entries(byTicket).sort((a, b) => b[1] - a[1]).map(([t, n]) => `${t} ${n}`).join(' · ')}`);
	if (!tag) {
		console.log('  no `bridge-audit-N` tag yet – tag the commit of a finished audit pass (`git tag bridge-audit-1`) for deltas');
		return;
	}
	try {
		const then = JSON.parse(git('show', `${tag}:${BASELINE}`));
		const thenLines = Object.values(then.budgets ?? {}).reduce((s, x) => s + x.lines, 0);
		console.log(`  since ${tag}: hits ${total(then.entries ?? then)} → ${total(doc.entries)}${thenLines ? `, budgeted lines ${thenLines} → ${lines}` : ''}`);
		const phrases = git('log', '--format=%h %s%n%b', `${tag}..HEAD`).split('\n').filter((l) => /twin of|as the native|kept in TS|trimmed copy|for now in the page/i.test(l));
		console.log(`  commit messages naming a twin since ${tag}: ${phrases.length}${phrases.length ? `\n    ${phrases.slice(0, 10).join('\n    ')}` : ''}`);
	} catch (e) {
		console.log(`  ${tag}: no baseline there (${e.message.split('\n')[0]})`);
	}
}

function main() {
	const args = process.argv.slice(2);
	const t = Date.now();
	const file = join(root, BASELINE);
	const doc = JSON.parse(read(file));
	const listed = JSON.parse(read(join(root, BRIDGE_FILES))).files;
	const { files, found, hits, excused, bad, sizes, unusedAllow } = scanAll(root, doc);
	if (args.includes('--shrink')) {
		writeFileSync(file, JSON.stringify(shrunk(found, doc, sizes, unusedAllow), null, '\t') + '\n');
		const bf = JSON.parse(read(join(root, BRIDGE_FILES)));
		bf.files = Object.fromEntries(Object.entries(bf.files).filter(([f]) => files.includes(f)));
		writeFileSync(join(root, BRIDGE_FILES), JSON.stringify(bf, null, '\t') + '\n');
		console.log(`check:core-only: ${BASELINE} and ${BRIDGE_FILES} shrunk to today's counts`);
		return;
	}
	const main_ = onMain();
	const errors = [
		...compare(found, doc.entries),
		...bad.map((b) => `${b.file}:${b.line}: core-only-ok without a ticket number (#N) – "${b.reason}"`),
		...overBudget(sizes, doc.budgets ?? {}),
		...purposes(files, listed),
		...dataFiles(root),
		...(main_.doc ? ratchet(main_.doc, doc, main_.approved) : [])
	];
	if (args.includes('--report') || args.includes('--audit')) {
		console.log(`baseline: ${Object.keys(doc.entries).length} files, ${total(doc.entries)} hits; ${files.length} bridge files`);
		const rows = Object.entries(doc.entries).map(([f, p]) => [f, Object.values(p).reduce((a, b) => a + b.count, 0), p]);
		for (const [f, n, p] of rows.sort((a, b) => b[1] - a[1])) {
			console.log(`  ${String(n).padStart(4)}  ${f}  ${Object.entries(p).map(([k, v]) => `${k} ${v.count} ${v.ticket}`).join(' · ')}`);
		}
		console.log(`core-only-ok lines: ${excused.length}`);
		for (const e of excused) console.log(`  ${e.file}:${e.line} ${e.pattern} – ${e.reason}`);
		if (args.includes('--hits')) for (const [f, hs] of Object.entries(hits)) for (const h of hs) console.log(`  ${f}:${h.line} ${h.rule}  ${h.code.slice(0, 110)}`);
		for (const a of unusedAllow) console.log(`  allow rule used ${a.used} of ${a.count ?? 1}: ${a.file} ${a.rule} "${a.match}" (--shrink)`);
	}
	if (args.includes('--audit')) audit(doc, sizes);
	const lower = Object.entries(doc.entries).some(([f, p]) => Object.entries(p).some(([k, v]) => (found[f]?.[k] ?? 0) < v.count)) || unusedAllow.length > 0;
	if (errors.length) {
		console.error(`check:core-only: ${errors.length} problem(s) – deck logic in the browser's TypeScript:`);
		for (const e of errors) console.error(`  ${e}`);
		console.error(
			[
				'',
				'What to do: move the logic into deck-core (Rust) and call it through live-core / the wasm, or take the value',
				'from the catalog (deck/catalog → kinds.json). Read agents/RULES.md "Code" → "Deck logic is Rust in the one universal',
				'core". Genuine adapter code (bytes in / out, drawing): mark the line `// core-only-ok: #<ticket> <why>`.',
				'Never add to the baseline to get past this – it only shrinks (the owner approves a growth in a commit message:',
				'`Owner-approved-baseline: <the owner\'s words, date>`).'
			].join('\n')
		);
		process.exit(1);
	}
	console.log(
		`check:core-only: ok – ${files.length} bridge files, ${total(doc.entries)} known hits in the baseline, ${excused.length} core-only-ok line(s)${lower ? ' (some counts fell: `npm run check:core-only -- --shrink`)' : ''}${main_.doc ? '' : ' (no baseline on main yet)'} · ${Date.now() - t} ms`
	);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
export { RULE_NAMES };
