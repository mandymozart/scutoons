// check-core-only.mjs (#357; owner 2026-11-03: "enforce it even for dumb agents"): the ratchet that keeps deck logic out
// of the browser's TypeScript. The rule (agents/DECISIONS.md 2026-11-02 "One universal core", RULES "Code"): all deck logic
// lives in deck-core (Rust, native + wasm); the browser only has thin adapters – bytes in and out, drawing, the UI's own
// state. This greps the browser bridge and its Svelte wrappers for the shapes runtime logic takes (scene literals, tempo
// and beat math, MIDI status bytes, texts the core produces, clamps, the lead rule, randomness, branching on kind names)
// and compares the hits per file and pattern with scripts/core-only-baseline.json, the known old cases, each with its
// ticket:
// - a hit not in the baseline, or more of them than it lists, fails;
// - the baseline may only shrink: an entry that grew or appeared against the baseline on `main` (the merge base) fails,
//   unless a commit since then carries `Owner-approved-baseline: <the owner's words, date>`;
// - a line marked `// core-only-ok: #N <reason>` (genuine adapter code) is no hit, but it is listed – the audit sees each.
//
//   npm run check:core-only                 the check (fast, no dependencies)
//   npm run check:core-only -- --report     the baseline's size, the top hits per file, every core-only-ok line
//   npm run check:core-only -- --shrink     write the baseline down to today's counts (never up)
import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync, writeFileSync, existsSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export const BASELINE = 'scripts/core-only-baseline.json';

/** where the browser's deck code lives: the bridge (live-core, live-deck, engine-host, live) and its Svelte wrappers */
export const ROOTS = [
	'shared/live-core/src',
	'shared/live-deck/src',
	'shared/engine-host/src',
	'shared/live/src',
	'viewer/src/lib/components/live-deck',
	'creator/src/lib/components/sound'
];

/** the shapes deck logic takes in TypeScript – each name is a baseline key */
export const PATTERNS = {
	'scene-literal': { re: /\bkind:\s*['"][a-z_]+['"]/g, what: 'a scene or patch written in TS (a node list) – scenes are TOML the core loads' },
	// (a frame rate's `/ 60` is no tempo; a minute's `/ 60_000` next to a beat is)
	'tempo-math': { re: /\bPPQN\b|\/\s*60_?000\b|\b60(?:_?000)?\s*\/\s*[\w.]*bpm|\bbpm\s*[*/]|[*/]\s*[\w.]*\bbpm\b|Math\.floor\(\s*beat/gi, what: 'tempo or beat arithmetic – the clock is deck-core\'s (tempo::Controller)' },
	// (a byte mask `& 0xff` is no MIDI)
	'midi-bytes': { re: /(?<!&\s*)\b0x(?:f[0-9a-e]|[89ab][0-9a-f])\b/gi, what: 'MIDI status bytes – MIDI is parsed and made in deck-core (tempo::midi_in, clock_out)' },
	'core-text': { re: /NO SHOW CLOCK|SHOW CLOCK SYNCING|NO (?:MIDI )?CLOCK|\bHOLDS\b|['"`\s]LEADS\b|SHOW \$\{/g, what: 'a status text the core produces (Controller::status) – show the core\'s line' },
	clamp: { re: /Math\.min\(\s*-?[\d.]+\s*,\s*Math\.max\(|Math\.max\(\s*-?[\d.]+\s*,\s*Math\.min\(/g, what: 'a clamp with fixed bounds – ranges and defaults are the catalog\'s (deck/catalog, kinds.json)' },
	'lead-rule': { re: /\bAdaptiveLead\b/g, what: 'the audio lead rule – deck-core\'s (lead.rs)' },
	random: { re: /Math\.random\(/g, what: 'randomness – deck behaviour must be the core\'s (seeded, the same on every target)' },
	'kind-branch': { re: /\bkind\s*[!=]==?\s*['"]/g, what: 'branching on a catalog kind name – kinds behave in the core' }
};

const ESCAPE = /core-only-ok:\s*(.*)$/;

/** a line without its comments (a `//` after a `:` or a quote is kept: URLs, strings) */
function code(line, state) {
	let s = line;
	if (state.block) {
		const end = s.indexOf('*/');
		if (end < 0) return '';
		s = s.slice(end + 2);
		state.block = false;
	}
	s = s.replace(/\/\*.*?\*\//g, '').replace(/<!--.*?-->/g, '');
	const open = s.indexOf('/*');
	if (open >= 0) {
		state.block = true;
		s = s.slice(0, open);
	}
	if (/^\s*\*/.test(s)) return '';
	return s.replace(/(^|[^:'"`\\\w])\/\/.*$/, '$1');
}

/** One file's text → its hits per pattern, its core-only-ok lines (with a ticket) and the ones without a ticket. */
export function scan(text) {
	const counts = {};
	const excused = [];
	const bad = [];
	const state = { block: false };
	const lines = text.split(/\r?\n/);
	lines.forEach((line, i) => {
		const esc = ESCAPE.exec(line) ?? (i > 0 ? ESCAPE.exec(lines[i - 1]) : null);
		const c = code(line, state);
		for (const [name, { re }] of Object.entries(PATTERNS)) {
			const n = (c.match(re) ?? []).length;
			if (!n) continue;
			if (esc && /#\d+/.test(esc[1])) excused.push({ line: i + 1, pattern: name, reason: esc[1].trim() });
			else {
				if (esc) bad.push({ line: i + 1, reason: esc[1].trim() });
				counts[name] = (counts[name] ?? 0) + n;
			}
		}
	});
	return { counts, excused, bad };
}

/** today's hits ({file: {pattern: n}}) against the baseline ({file: {pattern: {count, ticket}}}) → the failures */
export function compare(found, baseline) {
	const errors = [];
	for (const [file, counts] of Object.entries(found)) {
		for (const [pattern, n] of Object.entries(counts)) {
			const b = baseline[file]?.[pattern];
			if (!b) errors.push(`${file}: ${n} × ${pattern} – new (${PATTERNS[pattern].what})`);
			else if (n > b.count) errors.push(`${file}: ${n} × ${pattern}, the baseline allows ${b.count} (${b.ticket}) – ${PATTERNS[pattern].what}`);
		}
	}
	return errors;
}

/** the baseline against the one on `main`: it may only shrink (an approved commit may grow it) */
export function ratchet(base, current, approved) {
	if (approved) return [];
	const errors = [];
	for (const [file, pats] of Object.entries(current)) {
		for (const [pattern, { count }] of Object.entries(pats)) {
			const was = base[file]?.[pattern]?.count;
			if (was === undefined) errors.push(`baseline: ${file} ${pattern} is new – the baseline only shrinks`);
			else if (count > was) errors.push(`baseline: ${file} ${pattern} grew ${was} → ${count} – the baseline only shrinks`);
		}
	}
	return errors;
}

/** the baseline's entries that today's counts allow to shrink: {file: {pattern: {count, ticket}}} */
export function shrunk(found, baseline) {
	const out = {};
	for (const [file, pats] of Object.entries(baseline)) {
		for (const [pattern, b] of Object.entries(pats)) {
			const n = Math.min(b.count, found[file]?.[pattern] ?? 0);
			if (n > 0) (out[file] ??= {})[pattern] = { count: n, ticket: b.ticket };
		}
	}
	return out;
}

function files(dir) {
	if (!existsSync(dir)) return [];
	return readdirSync(dir).flatMap((n) => {
		const p = join(dir, n);
		if (statSync(p).isDirectory()) return n === 'test' || n === 'paraglide' ? [] : files(p);
		return /\.(ts|svelte)$/.test(n) && !/\.(test|d)\.ts$/.test(n) ? [p] : [];
	});
}

/** every scanned file's hits (generated files – `// GENERATED` on line 1 – are their generator's) */
export function scanAll(at = root) {
	const found = {};
	const excused = [];
	const bad = [];
	for (const r of ROOTS) {
		for (const p of files(join(at, r))) {
			const text = readFileSync(p, 'utf8');
			if (/^\/\/ GENERATED/.test(text)) continue;
			const f = relative(at, p).replaceAll('\\', '/');
			const s = scan(text);
			if (Object.keys(s.counts).length) found[f] = s.counts;
			excused.push(...s.excused.map((e) => ({ file: f, ...e })));
			bad.push(...s.bad.map((e) => ({ file: f, ...e })));
		}
	}
	return { found, excused, bad };
}

const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();

/** the baseline on `main`'s merge base and whether a commit since then carries the owner's approval (null: none there) */
function onMain() {
	try {
		const base = git('merge-base', 'HEAD', 'main');
		const approved = /Owner-approved-baseline:\s*\S/.test(git('log', '--format=%B', `${base}..HEAD`));
		try {
			return { entries: JSON.parse(git('show', `${base}:${BASELINE}`)).entries, approved };
		} catch {
			return { entries: null, approved };
		}
	} catch {
		return { entries: null, approved: false };
	}
}

function main() {
	const args = process.argv.slice(2);
	const t = Date.now();
	const file = join(root, BASELINE);
	const doc = JSON.parse(readFileSync(file, 'utf8'));
	const baseline = doc.entries;
	const { found, excused, bad } = scanAll();
	if (args.includes('--shrink')) {
		writeFileSync(file, JSON.stringify({ ...doc, entries: shrunk(found, baseline) }, null, '\t') + '\n');
		console.log(`check:core-only: ${BASELINE} shrunk to today's counts`);
		return;
	}
	const main_ = onMain();
	const errors = [
		...compare(found, baseline),
		...bad.map((b) => `${b.file}:${b.line}: core-only-ok without a ticket number (#N) – "${b.reason}"`),
		...(main_.entries ? ratchet(main_.entries, baseline, main_.approved) : [])
	];
	const total = Object.values(baseline).reduce((s, p) => s + Object.values(p).reduce((a, b) => a + b.count, 0), 0);
	if (args.includes('--report')) {
		console.log(`baseline: ${Object.keys(baseline).length} files, ${total} hits`);
		const rows = Object.entries(baseline).map(([f, p]) => [f, Object.values(p).reduce((a, b) => a + b.count, 0), p]);
		for (const [f, n, p] of rows.sort((a, b) => b[1] - a[1])) {
			console.log(`  ${String(n).padStart(4)}  ${f}  ${Object.entries(p).map(([k, v]) => `${k} ${v.count} ${v.ticket}`).join(' · ')}`);
		}
		console.log(`core-only-ok lines: ${excused.length}`);
		for (const e of excused) console.log(`  ${e.file}:${e.line} ${e.pattern} – ${e.reason}`);
	}
	const lower = Object.entries(baseline).some(([f, p]) => Object.entries(p).some(([k, v]) => (found[f]?.[k] ?? 0) < v.count));
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
	console.log(`check:core-only: ok – ${total} known hits in the baseline, ${excused.length} core-only-ok line(s)${lower ? ' (some counts fell: `npm run check:core-only -- --shrink`)' : ''}${main_.entries ? '' : ' (no baseline on main yet)'} · ${Date.now() - t} ms`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
