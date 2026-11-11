// The deck suite's CI on this machine (owner 2026-10-31: "the Rust runner definitely local now … build before every
// push, then push" – GitHub's macOS / Windows minutes cost 10× / 2×). The same steps as .github/workflows/deck.yml,
// which now runs by hand only (workflow_dispatch) – the GitHub variant for later, when we iterate less often.
//
//   npm run deck:ci                 everything below for this machine's platform
//   npm run deck:ci -- --quick      tests + clippy only (no release build, no wasm)
//   npm run deck:ci -- --no-release everything but the release build (fat LTO: minutes – the pre-push hook)
//   npm run deck:ci -- --if-changed <from> <to>   only when deck/ or shared/live* changed in that range (the pre-push hook)
//
// Steps: the bridge guard (check:core-only, check:deck-deps – #363), the headless deck host's scenarios (#363), the protocol schema current (shared/live), the kind catalog's generated files current (deck-space's deck-schema
// --check: kinds.json, the JSON Schema, shared/live/src/catalog.ts), live-core for wasm32 + simd128 (its gz size under the gate) + its TS tests, deck-engine for wasm32 + simd128 (its gz size under the gate), space-web for wasm32 (its gz size reported, no gate – owner 2026-11-11), deck-outlet for wasm32 (#378), deck-ui for wasm32 (#374) +
// shared/live-deck's tests, cargo build --release, cargo test (+ the stage's module tests), cargo clippy -D warnings (+ with the module feature). The Linux aarch64 cross build
// and the other platforms stay on GitHub (by hand) – this runs the host's native target.
import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const quick = args.includes('--quick');
const release = !args.includes('--no-release');
const WATCH = /^(deck\/|shared\/live\/|shared\/live-core\/|shared\/live-deck\/|\.github\/workflows\/deck\.yml$|scripts\/deck-ci\.mjs$|scripts\/(check-deck-deps|deck-deps|native-logic)\.)/;

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

/** the gz size gate of the engine's wasm (owner 2026-11-04: live-core, which links the engine since #320, may weigh 1 MB gz "for now"; was 500 KB on 2026-11-02) */
const ENGINE_GATE_KB = 1024;
/** a wasm's size, raw and gz; with `kb` a gate that fails above it. The browser deck (space-web) has no gate: owner
 * 2026-11-11, "Remove the browser check size limit. We will need to grow. Eventually" – its size is only reported */
function gate(file, kb) {
	const raw = readFileSync(file);
	const gz = gzipSync(raw, { level: 9 }).length;
	if (kb && gz > kb * 1024) throw new Error(`${file} is ${(gz / 1024).toFixed(0)} KB gz, over the ${kb} KB gate`);
	return `${(raw.length / 1024).toFixed(0)} KB raw, ${(gz / 1024).toFixed(0)} KB gz${kb ? `, gate ${kb} KB` : ''}`;
}

const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const steps = [
	// #357: no deck logic in the browser's TypeScript (the ratchet: scripts/core-only-baseline.json only shrinks)
	['🧱 core only', npm, ['run', 'check:core-only'], root],
	// #363: the core crates hold no platform library (cargo tree against scripts/deck-deps.json); main.rs and the native
	// files with deck logic only shrink (scripts/native-logic.json)
	['🧱 core crates platform-free', npm, ['run', 'check:deck-deps'], root],
	...(quick
		? []
		: [
				['📐 protocol schema current', npm, ['run', 'check', '-w', 'shared/live'], root],
				['📚 kind catalog current', 'cargo', ['run', '-q', '-p', 'deck-space', '--features', 'schema', '--bin', 'deck-schema', '--', '--check'], `${root}/deck`],
				// the browser deck's engine (#320): wasm32 + simd128 in its own target dir (shared/live-core/scripts/wasm.ts), gated
				[
					'🕸 live-core wasm32 + simd128',
					npm,
					['run', 'build:wasm', '-w', 'shared/live-core'],
					root,
					{},
					() => gate(`${process.env.CARGO_TARGET_DIR ?? `${root}/deck/target`}/live-core-simd/wasm32-unknown-unknown/release/live_core.wasm`, ENGINE_GATE_KB)
				],
				['🕸 live-core tests', npm, ['run', 'check', '-w', 'shared/live-core'], root],
				// the universal engine (#317) as a wasm32 module with simd128, in its own target dir (RUSTFLAGS would rebuild the main one)
				[
					'🕸 deck-engine wasm32 + simd128',
					'cargo',
					['build', '--release', '--target', 'wasm32-unknown-unknown', '-p', 'deck-engine-wasm', '--target-dir', 'target/engine-wasm'],
					`${root}/deck`,
					{ RUSTFLAGS: '-C target-feature=+simd128' },
					() => gate(`${root}/deck/target/engine-wasm/wasm32-unknown-unknown/release/deck_engine_wasm.wasm`, ENGINE_GATE_KB)
				],
				// the browser deck's 3D and own UI (#198, #485), gated: the phone downloads it
				[
					'🧊 space-web wasm32',
					'cargo',
					['build', '--profile', 'wasm-web', '--target', 'wasm32-unknown-unknown', '-p', 'space-web'],
					`${root}/deck`,
					{},
					() => gate(`${process.env.CARGO_TARGET_DIR ?? `${root}/deck/target`}/wasm32-unknown-unknown/wasm-web/space_web.wasm`, SPACE_WEB_GATE_KB)
				],
				// #378: the Live outlet's logic (plan, composer, crossfade, text layout, routing) builds for the browser – a build check, not a feature
				['🎭 deck-outlet wasm32', 'cargo', ['build', '--target', 'wasm32-unknown-unknown', '-p', 'deck-outlet'], `${root}/deck`],
				// #89: the show core builds for the browser – a build check
				['🎼 deck-show wasm32', 'cargo', ['build', '--target', 'wasm32-unknown-unknown', '-p', 'deck-show'], `${root}/deck`],
				// #374: the deck's UI model (the HUD's lines, the bar's items, the F1 mode) builds for the browser – a build check
				['🪟 deck-ui wasm32', 'cargo', ['build', '--target', 'wasm32-unknown-unknown', '-p', 'deck-ui'], `${root}/deck`],
				// live-deck's in-process relay tests spawn deck/target/debug/relay.exe: a stale binary fails them after a relay change (the P8 merge)
				['🧊 relay binary', 'cargo', ['build', '-p', 'relay'], `${root}/deck`],
				// #366: the relay client's wasm (relay-web) – live-deck's tests run the core's scripted links through client.ts
				['🛰 relay-web wasm32', npm, ['run', 'build:wasm', '-w', 'shared/live-deck', '--', '--relay'], root],
				['🧊 live-deck tests', npm, ['run', 'check', '-w', 'shared/live-deck'], root],
				// #321: the show clock's LAN measurement (the owner's recording → the decks' offsets)
				['⏱ clock measurement tool', npm, ['run', 'check', '-w', 'tools/clock-measure'], root],
				...(release ? [['🦀 release build', 'cargo', ['build', '--release'], `${root}/deck`]] : [])
			]),
	// #363: the headless deck host's scenarios, one line each (pass / xfail with its move ticket / FAIL); the native ≡ wasm32
	// ≡ simd128 run of the same scenarios is deck-headless's tests/wasm.rs in the workspace's tests below
	['🎧 headless scenarios', 'cargo', ['run', '-q', '-p', 'deck-headless', '--bin', 'deck-scenarios'], `${root}/deck`],
	['🦀 tests', 'cargo', ['test', '--workspace'], `${root}/deck`],
	// the stage's `modules` feature is not in its default, so the workspace run never builds it (#305)
	['🧩 module tests', 'cargo', ['test', '-p', 'deck-native', '--features', 'modules', '--test', 'modules'], `${root}/deck`],
	// #342: the engine's module node (channels, control outputs, control-domain modules, the event host) behind stand-ins
	['🧩 engine module tests', 'cargo', ['test', '-p', 'deck-engine', '--features', 'events,modules', '--lib', 'module'], `${root}/deck`],
	['🦀 clippy', 'cargo', ['clippy', '--workspace', '--all-targets', '--', '-D', 'warnings'], `${root}/deck`],
	['🧩 module clippy', 'cargo', ['clippy', '-p', 'deck-native', '--features', 'modules', '--all-targets', '--', '-D', 'warnings'], `${root}/deck`],
	['🧩 engine module clippy', 'cargo', ['clippy', '-p', 'deck-engine', '--features', 'events,modules', '--all-targets', '--', '-D', 'warnings'], `${root}/deck`]
];

const started = Date.now();
for (const [name, cmd, argv, cwd, env, after] of steps) {
	const t = Date.now();
	process.stdout.write(`deck:ci ${name} … `);
	const r = spawnSync(cmd, argv, { cwd, encoding: 'utf8', env: { ...process.env, ...env }, shell: process.platform === 'win32' && cmd.endsWith('.cmd') });
	let failed = r.status !== 0;
	let note = '';
	if (!failed && after) {
		try {
			note = ` – ${after()}`;
		} catch (e) {
			r.stdout = `${r.stdout ?? ''}
${e.message}`;
			failed = true;
		}
	}
	if (failed) {
		console.log('FAILED');
		console.log((r.stdout ?? '') + (r.stderr ?? ''));
		console.log(`deck:ci failed at "${name}" – fix it before pushing (or push with --no-verify if you must)`);
		process.exit(1);
	}
	console.log(`ok (${((Date.now() - t) / 1000).toFixed(0)} s)${note}`);
}
console.log(`deck:ci green in ${((Date.now() - started) / 1000).toFixed(0)} s`);
