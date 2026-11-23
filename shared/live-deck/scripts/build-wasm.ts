// `npm run build:wasm -w shared/live-deck` (#198, #366): the browser deck's wasm modules, built from deck/ – space-web (deck-space's
// being and set, wgpu) and relay-web (deck-space's relay client: the deck page, the vote screens) by cargo at the `wasm-web` profile
// (opt-level s) + wasm-bindgen --target web (the CLI is the crates' version: `cargo install wasm-bindgen-cli --version 0.2.129`);
// live-core (the sound, #320) as shared/live-core builds it (no bindgen). `-- --relay`: relay-web alone (deck:ci, before live-deck's
// tests). `-- --sandbox`: a dev build – space-web's `?scene=` plays the sandbox scenes too (#427, deck/native/scenes/sandbox; the
// published build carries none). `-- --spores`: spores-web alone (#763). The files go to shared/live-deck/wasm/ (deck/lab's build
// copies them), viewer/static/live/ and creator/static/live/ (#298), all git-ignored. The deck's target folder: deckTarget() (#526).
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync, readFileSync, statSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { fileURLToPath } from 'node:url';
import { buildCommand, deckTarget, wasmPath } from '@scutoons/live-core/wasm-path';

const here = (p: string) => fileURLToPath(new URL(p, import.meta.url));
const deck = here('../../../deck/');
const web = `${deckTarget()}/wasm32-unknown-unknown/wasm-web`;
const run = (cmd: string, args: string[], env = process.env) => execFileSync(cmd, args, { stdio: 'inherit', cwd: deck, env });
const out = here('../wasm/');
mkdirSync(out, { recursive: true });
const crates = process.argv.includes('--relay') ? ['relay_web'] : process.argv.includes('--spores') ? ['spores_web'] : ['relay_web', 'space_web', 'spores_web'];
const files: string[] = [];
for (const c of crates) {
	const dev = c === 'space_web' && process.argv.includes('--sandbox') ? ['--features', 'sandbox'] : [];
	run('cargo', ['build', '--profile', 'wasm-web', '--target', 'wasm32-unknown-unknown', '-p', c.replace('_', '-'), ...dev], c === 'relay_web' ? process.env : { ...process.env, RUSTC_WRAPPER: '' }); // #526: no sccache – it expands web-sys's @argfile past Windows' 32 K command line (GOTCHAS)
	run('wasm-bindgen', ['--target', 'web', '--no-typescript', '--out-dir', out, `${web}/${c}.wasm`]);
	files.push(`${c}.js`, `${c}_bg.wasm`);
}
if (crates.includes('space_web')) {
	const live = buildCommand();
	run(live.cmd, live.args, live.env);
	copyFileSync(wasmPath(), `${out}live_core.wasm`);
	files.push('live_core.wasm');
}
const apps = ['viewer', 'creator', 'admin'].map((app) => here(`../../../${app}/static/live/`));
for (const dir of apps) {
	mkdirSync(dir, { recursive: true });
	for (const f of files) copyFileSync(`${out}${f}`, `${dir}${f}`);
}
const kb = (n: number) => `${(n / 1024).toFixed(0)} KB`;
for (const f of files) console.log(`${f}: ${kb(statSync(`${out}${f}`).size)} raw, ${kb(gzipSync(readFileSync(`${out}${f}`), { level: 9 }).length)} gzip`);
console.log(`→ ${out} and ${apps.join(' and ')}`);
