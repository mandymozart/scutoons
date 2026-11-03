// check-deck-deps.mjs (#363 part B; owner 2026-11-03: "my hope those that shouldnt will fail if the native deck is properly
// encapsulated"): the structural half of the bridge guard, on the Rust side. check:core-only keeps deck logic out of the
// browser's TypeScript; this keeps the platform out of the core and the logic out of the native platform crate:
// - the core crates (scripts/deck-deps.json `crates`, with the features they are checked with) depend on no platform
//   library: every crate in their normal dependency tree (`cargo tree`, all targets) is in `allow` with a reason, none in
//   `deny` (audio, window, GPU, sockets, devices, async runtimes, HTTP, browser glue). A platform need belongs to an
//   adapter crate (deck-native, space-web, bridge, relay, the adapters); the justified exceptions are in the file;
// - deck-native's main.rs and the platform files that still hold deck logic (scripts/native-logic.json, BRIDGE-AUDIT.md
//   §1) only shrink: main.rs gains no function and no line, each listed file no line; a logic function that left main.rs
//   is a move done (--shrink drops it). Growth needs `Owner-approved-baseline:` in a commit since main.
//
//   npm run check:deck-deps                 the check (cargo tree, no build; a few seconds)
//   npm run check:deck-deps -- --shrink     write native-logic.json down to today's numbers
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export const DEPS = 'scripts/deck-deps.json';
export const NATIVE = 'scripts/native-logic.json';

/** a crate's normal dependency tree under `args` (features) for all targets → the crate names, itself excluded */
export function tree(crate, args, deck = join(root, 'deck')) {
	const out = execFileSync('cargo', ['tree', '-q', '--offline', '-p', crate, ...args, '-e', 'normal', '--prefix', 'none', '-f', '{p}', '--target', 'all'], {
		cwd: deck,
		encoding: 'utf8',
		stdio: ['ignore', 'pipe', 'pipe']
	});
	return [...new Set(out.split('\n').map((l) => l.trim().split(' ')[0]).filter(Boolean))].filter((n) => n !== crate).sort();
}

/** one core crate's dependencies against the allow and deny lists → the failures */
export function judge(crate, names, { allow, deny }) {
	const errors = [];
	for (const n of names) {
		if (deny[n]) errors.push(`${crate} depends on ${n} – a platform library (${deny[n]}); a core crate holds no platform: put it in an adapter crate (deck-native, space-web, bridge, an adapter)`);
		else if (!allow[n]) errors.push(`${crate} depends on ${n}, which ${DEPS} does not list – a new dependency of a core crate needs a line in \`allow\` with a reason (and is never a platform library)`);
	}
	return errors;
}

/** a file's lines as wc -l counts them */
export const lineCount = (text) => text.split('\n').length - (text.endsWith('\n') ? 1 : 0);

/** the functions a Rust file defines (names, in order) */
export function functions(text) {
	return [...text.matchAll(/^\s*(?:pub(?:\([a-z]+\))?\s+)?(?:const\s+|async\s+|unsafe\s+)*fn\s+(\w+)/gm)].map((m) => m[1]);
}

/** deck-native's main.rs and the logic files against native-logic.json → {errors, notes} */
export function native(base, read) {
	const errors = [];
	const notes = [];
	const text = read(base.main.file);
	const lines = lineCount(text);
	if (lines > base.main.lines) errors.push(`${base.main.file}: ${lines} lines, it may have ${base.main.lines} – main.rs is the platform's entry; logic goes into a core crate (#374)`);
	const fns = functions(text);
	for (const f of fns) if (!base.main.functions.includes(f)) errors.push(`${base.main.file}: fn ${f} is new – main.rs gains no functions; put the logic into a core crate (#374) or an adapter module`);
	for (const [f, why] of Object.entries(base.main.logic)) if (!fns.includes(f)) notes.push(`main.rs: ${f} left (${why.split(' – ')[0]}) – drop its line: npm run check:deck-deps -- --shrink`);
	for (const [file, { lines: max, ticket }] of Object.entries(base.files)) {
		if (!existsSync(join(root, file))) {
			notes.push(`${file} is gone (${ticket}) – --shrink drops it`);
			continue;
		}
		const n = lineCount(read(file));
		if (n > max) errors.push(`${file}: ${n} lines, it may have ${max} – it holds deck logic that ${ticket} moves out; don't grow it`);
	}
	return { errors, notes, lines, fns };
}

/** native-logic.json and the deny list against main's: they only shrink (an approved commit may grow them) */
export function ratchet(mainNative, nowNative, mainDeps, nowDeps, approved) {
	if (approved) return [];
	const errors = [];
	if (mainNative) {
		if (nowNative.main.lines > mainNative.main.lines) errors.push(`${NATIVE}: main.rs may have ${mainNative.main.lines} lines on main, ${nowNative.main.lines} here – it only shrinks`);
		for (const f of nowNative.main.functions) if (!mainNative.main.functions.includes(f)) errors.push(`${NATIVE}: fn ${f} is new in main.rs's list – it only shrinks`);
		for (const [file, b] of Object.entries(nowNative.files)) {
			const was = mainNative.files[file]?.lines;
			if (was === undefined || b.lines > was) errors.push(`${NATIVE}: ${file} ${was ?? 'new'} → ${b.lines} lines – it only shrinks`);
		}
	}
	if (mainDeps) for (const n of Object.keys(mainDeps.deny)) if (!nowDeps.deny[n]) errors.push(`${DEPS}: ${n} left the deny list – a platform library stays denied`);
	return errors;
}

/** native-logic.json written down to today's numbers */
export function shrunk(base, now, read) {
	const files = {};
	for (const [file, b] of Object.entries(base.files)) if (existsSync(join(root, file))) files[file] = { ...b, lines: Math.min(b.lines, lineCount(read(file))) };
	const logic = Object.fromEntries(Object.entries(base.main.logic).filter(([f]) => now.fns.includes(f)));
	return { ...base, main: { ...base.main, lines: Math.min(base.main.lines, now.lines), functions: base.main.functions.filter((f) => now.fns.includes(f)), logic }, files };
}

const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();

function onMain(file) {
	try {
		const base = git('merge-base', 'HEAD', 'main');
		const approved = /Owner-approved-baseline:\s*\S/.test(git('log', '--format=%B', `${base}..HEAD`));
		try {
			return { doc: JSON.parse(git('show', `${base}:${file}`)), approved };
		} catch {
			return { doc: null, approved };
		}
	} catch {
		return { doc: null, approved: false };
	}
}

function main() {
	const t = Date.now();
	const read = (f) => readFileSync(join(root, f), 'utf8');
	const deps = JSON.parse(read(DEPS));
	const base = JSON.parse(read(NATIVE));
	const errors = [];
	let checked = 0;
	for (const c of deps.crates) {
		let names;
		try {
			names = tree(c.crate, c.args);
		} catch (e) {
			errors.push(`${c.crate}: cargo tree failed – ${String(e.stderr || e.message).trim().split('\n')[0]}`);
			continue;
		}
		checked++;
		errors.push(...judge(c.crate, names, deps));
	}
	const n = native(base, read);
	if (process.argv.includes('--shrink')) {
		writeFileSync(join(root, NATIVE), JSON.stringify(shrunk(base, n, read), null, '\t') + '\n');
		console.log(`check:deck-deps: ${NATIVE} shrunk to today's numbers`);
		return;
	}
	errors.push(...n.errors);
	const mn = onMain(NATIVE);
	const md = onMain(DEPS);
	errors.push(...ratchet(mn.doc, base, md.doc, deps, mn.approved));
	for (const note of n.notes) console.log(`  note: ${note}`);
	if (errors.length) {
		console.error(`check:deck-deps: ${errors.length} problem(s) – the platform in a core crate, or logic growing in the platform crate:`);
		for (const e of errors) console.error(`  ${e}`);
		process.exit(1);
	}
	console.log(`check:deck-deps: ok – ${checked} core crates without a platform library, main.rs ${n.lines} lines / ${n.fns.length} fns (${Object.keys(base.main.logic).length} hold deck logic) · ${Date.now() - t} ms`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
