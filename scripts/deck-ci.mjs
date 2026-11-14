// The deck suite's CI on this machine (owner 2026-10-31: "the Rust runner definitely local now … build before every
// push, then push" – GitHub's macOS / Windows minutes cost 10× / 2×). The same steps as .github/workflows/deck.yml,
// which now runs by hand only (workflow_dispatch) – the GitHub variant for later, when we iterate less often.
//
//   npm run deck:ci                 the default tier: everything below, the release profile only checked (cargo check --release) –
//                                   before every merge / push of deck work (worker, orchestrator, the pre-push hook)
//   npm run deck:ci -- --full       the full tier: the default tier with the real fat-LTO release build instead of the check –
//                                   at the end of every shift and before a deck release (#524)
//   npm run deck:ci -- --quick      the guards, tests + clippy only (no release, no wasm)
//   npm run deck:ci -- --fresh      ignore the green stamps (also DECK_CI_FRESH=1); new ones are written
//   npm run deck:ci -- --plan       print which steps would run or be skipped, run nothing
//   npm run deck:ci -- --if-changed <from> <to>   only when a WATCH path changed in that range (the pre-push hook)
//   (--no-release: the default tier – kept so a pre-push hook installed before #524 keeps working)
//
// Green stamps (#524, scripts/deck-ci-stamps.ts): a green step is stamped with its content key (the watched files at HEAD,
// the step, the tier, the toolchain) in the repo's common git dir; a later run on the same clean tree – another worktree,
// the orchestrator, the pre-push hook – skips it and names the stamp's date. Uncommitted or untracked changes under a
// watched path: every step runs, nothing is stamped.
//
// Steps: the bridge guard (check:core-only, check:deck-deps – #363), the protocol schema current (shared/live), the kind catalog's generated files current (deck-space's deck-schema
// --check: kinds.json, the JSON Schema, shared/live/src/catalog.ts), live-core for wasm32 + simd128 (its gz size under the gate) + its TS tests, deck-engine for wasm32 + simd128 (its gz size under the gate), space-web for wasm32 (its gz size reported, no gate – owner 2026-11-11), deck-outlet + deck-ui for wasm32 (#378, #374), deck-show for wasm32 +
// shared/live-deck's tests, the release profile (checked; built in the full tier), cargo test (the headless deck host's scenarios run in it:
// headless/tests/scenarios.rs) + the stage's and the engine's module tests, one cargo clippy -D warnings over the module feature union. The Linux aarch64 cross build
// and the other platforms stay on GitHub (by hand) – this runs the host's native target.
import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

// the stamps are TypeScript (RULES J1): run again with type stripping when started by a plain `node` (an older hook)
if (!process.execArgv.some((a) => /^--experimental-(transform|strip)-types$/.test(a))) {
	const r = spawnSync(process.execPath, ['--experimental-transform-types', '--no-warnings', fileURLToPath(import.meta.url), ...process.argv.slice(2)], { stdio: ['ignore', 'inherit', 'inherit'] });
	process.exit(r.status ?? 1);
}
const { WATCH, Stamps, contentKey, decide, gitState, stampsDir, toolchain, watched } = await import('./deck-ci-stamps.ts');
const { ensure, layout } = await import('./deck-target.ts');
const { deckTarget, wasmPath } = await import('../shared/live-core/scripts/wasm.ts');

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const tier = args.includes('--quick') ? 'quick' : args.includes('--full') ? 'full' : 'default';
const fresh = args.includes('--fresh') || !!process.env.DECK_CI_FRESH;
const planOnly = args.includes('--plan');

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
	if (!files.some((f) => watched(f, WATCH))) {
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

// the deck's builds (#526): a worktree's in <git common dir>/deck-target/<id> (its .cargo/config.toml, written here too in
// case npm install ran before #526), the main checkout's in deck/target. Every path below is cargo's own answer
let where;
try {
	where = ensure(layout(root));
} catch (e) {
	where = `not set up (${e.message.split('\n')[0]})`;
}
const target = deckTarget();
const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const deck = `${root}/deck`;
/** an npm step's inputs beyond WATCH (its stamp key) */
const NODE = ['package.json', 'package-lock.json'];
/** the stage's and the engine's module features, not in their defaults: one feature union for clippy (#305, #342, #524) */
const MODULES = ['--features', 'deck-native/modules,deck-engine/events,deck-engine/modules'];
/** { name, cmd, argv, cwd, env?, after?, paths? (beyond WATCH), stamp? (false: always runs) } */
const step = (name, cmd, argv, cwd, more = {}) => ({ name, cmd, argv, cwd, ...more });
const steps = [
	// #357: no deck logic in the browser's TypeScript (the ratchet: scripts/core-only-baseline.json only shrinks). Never
	// stamped: it scans viewer / creator / admin / tools and compares with main's baseline – more than the watched tree
	step('🧱 core only', npm, ['run', 'check:core-only'], root, { stamp: false }),
	// #363: the core crates hold no platform library (cargo tree against scripts/deck-deps.json); main.rs and the native
	// files with deck logic only shrink (scripts/native-logic.json)
	step('🧱 core crates platform-free', npm, ['run', 'check:deck-deps'], root, { paths: NODE }),
	...(tier === 'quick'
		? []
		: [
				step('📐 protocol schema current', npm, ['run', 'check', '-w', 'shared/live'], root, { paths: NODE }),
				step('📚 kind catalog current', 'cargo', ['run', '-q', '-p', 'deck-space', '--features', 'schema', '--bin', 'deck-schema', '--', '--check'], deck),
				// the browser deck's engine (#320): wasm32 + simd128 in its own target dir (shared/live-core/scripts/wasm.ts), gated
				step('🕸 live-core wasm32 + simd128', npm, ['run', 'build:wasm', '-w', 'shared/live-core'], root, {
					paths: NODE,
					after: () => gate(wasmPath(), ENGINE_GATE_KB)
				}),
				step('🕸 live-core tests', npm, ['run', 'check', '-w', 'shared/live-core'], root, { paths: NODE }),
				// the universal engine (#317) as a wasm32 module with simd128, in its own target dir (RUSTFLAGS would rebuild the main one)
				step('🕸 deck-engine wasm32 + simd128', 'cargo', ['build', '--release', '--target', 'wasm32-unknown-unknown', '-p', 'deck-engine-wasm', '--target-dir', `${target}/engine-wasm`], deck, {
					env: { RUSTFLAGS: '-C target-feature=+simd128' },
					after: () => gate(`${target}/engine-wasm/wasm32-unknown-unknown/release/deck_engine_wasm.wasm`, ENGINE_GATE_KB)
				}),
				// the browser deck's 3D and own UI (#198, #485): its size reported, no gate (owner 2026-11-11)
				step('🧊 space-web wasm32', 'cargo', ['build', '--profile', 'wasm-web', '--target', 'wasm32-unknown-unknown', '-p', 'space-web'], deck, {
					after: () => gate(`${target}/wasm32-unknown-unknown/wasm-web/space_web.wasm`)
				}),
				// build checks for the browser, one build: the Live outlet's logic (plan, composer, crossfade, text layout, routing; #378)
				// and the deck's UI model (the HUD's lines, the bar's items, the F1 mode; #374)
				step('🎭 deck-outlet + deck-ui wasm32', 'cargo', ['build', '--target', 'wasm32-unknown-unknown', '-p', 'deck-outlet', '-p', 'deck-ui'], deck),
				// #89: the show core builds for the browser – a build check
				step('🎼 deck-show wasm32', 'cargo', ['build', '--target', 'wasm32-unknown-unknown', '-p', 'deck-show'], deck),
				// live-deck's in-process relay tests spawn debug/relay.exe of the deck's target: a stale binary fails them after a relay change (the P8 merge)
				step('🧊 relay binary', 'cargo', ['build', '-p', 'relay'], deck),
				// #366: the relay client's wasm (relay-web) – live-deck's tests run the core's scripted links through client.ts
				step('🛰 relay-web wasm32', npm, ['run', 'build:wasm', '-w', 'shared/live-deck', '--', '--relay'], root, { paths: NODE }),
				step('🧊 live-deck tests', npm, ['run', 'check', '-w', 'shared/live-deck'], root, { paths: NODE }),
				// #321: the show clock's LAN measurement (the owner's recording → the decks' offsets)
				step('⏱ clock measurement tool', npm, ['run', 'check', '-w', 'tools/clock-measure'], root, { paths: [...NODE, 'tools/clock-measure'] }),
				// #524: the release profile is checked per merge; the fat-LTO build (minutes) only in the full tier
				tier === 'full'
					? step('🦀 release build', 'cargo', ['build', '--release'], deck)
					: step('🦀 release check', 'cargo', ['check', '--release', '--workspace'], deck)
			]),
	// the headless deck host's scenarios run here (headless/tests/scenarios.rs, the same files and the same pass / xfail / FAIL
	// as `cargo run -p deck-headless --bin deck-scenarios`), native ≡ wasm32 ≡ simd128 in its tests/wasm.rs
	step('🦀 tests', 'cargo', ['test', '--workspace'], deck),
	// the stage's `modules` feature is not in its default, so the workspace run never builds it (#305). Not in the feature union
	// of the tests (#524): with deck-native/modules the conformance tests keep the module nodes (deck_native::modules::carried_part)
	// and their graph fails ("unknown kind 'module:stepseq'", control_rate) – that is a change of the tests, not of deck:ci
	step('🧩 module tests', 'cargo', ['test', '-p', 'deck-native', '--features', 'modules', '--test', 'modules'], deck),
	// #342: the engine's module node (channels, control outputs, control-domain modules, the event host) behind stand-ins
	step('🧩 engine module tests', 'cargo', ['test', '-p', 'deck-engine', '--features', 'events,modules', '--lib', 'module'], deck),
	// one clippy over the feature union (#524; was three runs: workspace, deck-native modules, deck-engine events + modules)
	step('🦀 clippy', 'cargo', ['clippy', '--workspace', '--all-targets', ...MODULES, '--', '-D', 'warnings'], deck)
];

const stamps = new Stamps(stampsDir(root));
stamps.prune();
const chain = toolchain(deck);
let state = gitState(root);
const head = state.head;
const keyOf = (s, st) => contentKey({ step: s.name, tier, toolchain: chain, tree: st.tree, paths: [...WATCH, ...(s.paths ?? [])] });
const plans = steps.map((s) => {
	const key = keyOf(s, state);
	return { key, plan: decide({ dirty: state.dirty, paths: [...WATCH, ...(s.paths ?? [])], stampable: s.stamp !== false, fresh, found: stamps.find(key) }) };
});
const dirtyWatched = state.dirty.find((f) => watched(f, WATCH));
console.log(
	`deck:ci ${tier} tier at ${head.slice(0, 8)}${fresh ? ' – fresh: stamps ignored' : ''}${dirtyWatched ? ` – uncommitted change (${dirtyWatched}): every step runs, nothing is stamped` : ''}`
);
console.log(`deck:ci builds in ${target.replaceAll('\\', '/')} – ${where}`);

if (planOnly) {
	for (const [i, s] of steps.entries()) {
		const p = plans[i].plan;
		console.log(`deck:ci ${s.name} … ${p.action === 'skip' ? `would skip – green on this tree ${when(p.stamp)}` : `would run – ${p.why}`}`);
	}
	process.exit(0);
}

function when(st) {
	return `${st.date.slice(0, 16).replace('T', ' ')} (${st.seconds} s then)`;
}

const started = Date.now();
let skipped = 0;
for (const [i, { name, cmd, argv, cwd, env, after, paths }] of steps.entries()) {
	const { key, plan } = plans[i];
	process.stdout.write(`deck:ci ${name} … `);
	if (plan.action === 'skip') {
		skipped++;
		console.log(`skipped – green on this tree ${when(plan.stamp)}`);
		continue;
	}
	const t = Date.now();
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
	const seconds = Math.round((Date.now() - t) / 1000);
	let stamped = '';
	if (plan.stamp) {
		// only when the tree is still the one the step started on: same HEAD, same key, still clean
		state = gitState(root);
		const again = decide({ dirty: state.dirty, paths: [...WATCH, ...(paths ?? [])], stampable: true, fresh: true });
		if (state.head === head && keyOf(steps[i], state) === key && again.stamp) {
			stamps.write(key, { step: name, tier, date: new Date().toISOString(), seconds, head, toolchain: chain });
		} else stamped = ' – not stamped: the tree changed during the step';
	}
	console.log(`ok (${seconds} s)${note}${stamped}`);
}
console.log(`deck:ci green in ${((Date.now() - started) / 1000).toFixed(0)} s${skipped ? ` (${skipped} of ${steps.length} steps skipped by their green stamps)` : ''}`);
