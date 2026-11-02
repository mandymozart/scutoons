// The deck suite's CI on this machine (owner 2026-10-31: "the Rust runner definitely local now … build before every
// push, then push" – GitHub's macOS / Windows minutes cost 10× / 2×). The same steps as .github/workflows/deck.yml,
// which now runs by hand only (workflow_dispatch) – the GitHub variant for later, when we iterate less often.
//
//   npm run deck:ci                 everything below for this machine's platform
//   npm run deck:ci -- --quick      tests + clippy only (no release build, no wasm)
//   npm run deck:ci -- --no-release everything but the release build (fat LTO: minutes – the pre-push hook)
//   npm run deck:ci -- --if-changed <from> <to>   only when deck/ or shared/live* changed in that range (the pre-push hook)
//
// Steps: the protocol schema current (shared/live), the kind catalog's generated files current (deck-space's deck-schema
// --check: kinds.json, NodeKinds.php, the JSON Schema, shared/live/src/catalog.ts), live-core for wasm32 + its TS tests, space-web for wasm32 +
// shared/live-deck's tests, cargo build --release, cargo test (+ the stage's module tests), cargo clippy -D warnings (+ with the module feature). The Linux aarch64 cross build
// and the other platforms stay on GitHub (by hand) – this runs the host's native target.
import { execFileSync, spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const quick = args.includes('--quick');
const release = !args.includes('--no-release');
const WATCH = /^(deck\/|shared\/live\/|shared\/live-core\/|shared\/live-deck\/|\.github\/workflows\/deck\.yml$|scripts\/deck-ci\.mjs$)/;

const changed = args.indexOf('--if-changed');
if (changed >= 0) {
	const [from, to] = args.slice(changed + 1, changed + 3);
	// a new branch on the remote (from = zeros): compare with origin/main
	const base = !from || /^0+$/.test(from) ? 'origin/main' : from;
	let files = [];
	try {
		files = execFileSync('git', ['diff', '--name-only', `${base}..${to ?? 'HEAD'}`], { cwd: root, encoding: 'utf8' }).split('\n').filter(Boolean);
	} catch {
		files = ['deck/'];
	}
	if (!files.some((f) => WATCH.test(f))) {
		console.log('deck:ci – nothing in deck/ or shared/live* changed, skipped');
		process.exit(0);
	}
}

const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const steps = [
	...(quick
		? []
		: [
				['📐 protocol schema current', npm, ['run', 'check', '-w', 'shared/live'], root],
				['📚 kind catalog current', 'cargo', ['run', '-q', '-p', 'deck-space', '--features', 'schema', '--bin', 'deck-schema', '--', '--check'], `${root}/deck`],
				['🕸 live-core wasm32', npm, ['run', 'build:wasm', '-w', 'shared/live-core'], root],
				['🕸 live-core tests', npm, ['run', 'check', '-w', 'shared/live-core'], root],
				['🧊 space-web wasm32', 'cargo', ['build', '--profile', 'wasm-web', '--target', 'wasm32-unknown-unknown', '-p', 'space-web'], `${root}/deck`],
				['🧊 live-deck tests', npm, ['run', 'check', '-w', 'shared/live-deck'], root],
				...(release ? [['🦀 release build', 'cargo', ['build', '--release'], `${root}/deck`]] : [])
			]),
	['🦀 tests', 'cargo', ['test', '--workspace'], `${root}/deck`],
	// the stage's `modules` feature is not in its default, so the workspace run never builds it (#305)
	['🧩 module tests', 'cargo', ['test', '-p', 'deck-stage', '--features', 'modules', '--test', 'modules'], `${root}/deck`],
	['🦀 clippy', 'cargo', ['clippy', '--workspace', '--all-targets', '--', '-D', 'warnings'], `${root}/deck`],
	['🧩 module clippy', 'cargo', ['clippy', '-p', 'deck-stage', '--features', 'modules', '--all-targets', '--', '-D', 'warnings'], `${root}/deck`]
];

const started = Date.now();
for (const [name, cmd, argv, cwd] of steps) {
	const t = Date.now();
	process.stdout.write(`deck:ci ${name} … `);
	const r = spawnSync(cmd, argv, { cwd, encoding: 'utf8', shell: process.platform === 'win32' && cmd.endsWith('.cmd') });
	if (r.status !== 0) {
		console.log('FAILED');
		console.log((r.stdout ?? '') + (r.stderr ?? ''));
		console.log(`deck:ci failed at "${name}" – fix it before pushing (or push with --no-verify if you must)`);
		process.exit(1);
	}
	console.log(`ok (${((Date.now() - t) / 1000).toFixed(0)} s)`);
}
console.log(`deck:ci green in ${((Date.now() - started) / 1000).toFixed(0)} s`);
