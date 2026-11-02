// `npm run build:wasm -w shared/live-deck` (#198): the browser deck's two wasm modules, built from deck/ –
// - space-web (deck-space's being and set, wgpu): cargo for wasm32, then wasm-bindgen --target web (the CLI must be the
//   crate's version, 0.2.129: `cargo install wasm-bindgen-cli --version 0.2.129`);
// - live-core (the sound): cargo for wasm32 (no bindgen).
// The files go to shared/live-deck/wasm/ (deck/lab's build copies them), viewer/static/live/ (the viewer's /live/deck loads
// them at run time) and creator/static/live/ (the engine host in the creator, #298); all git-ignored. CARGO_TARGET_DIR is honoured (default deck/target).
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync, readFileSync, statSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { fileURLToPath } from 'node:url';

const here = (p: string) => fileURLToPath(new URL(p, import.meta.url));
const deck = here('../../../deck/');
const target = process.env.CARGO_TARGET_DIR ?? `${deck}target`;
const release = `${target}/wasm32-unknown-unknown/release`;
// space-web at deck/Cargo.toml's `wasm-web` profile (opt-level s: ~10 % smaller), live-core as shared/live-core builds it
const web = `${target}/wasm32-unknown-unknown/wasm-web`;
const run = (cmd: string, args: string[]) => execFileSync(cmd, args, { stdio: 'inherit', cwd: deck });

run('cargo', ['build', '--profile', 'wasm-web', '--target', 'wasm32-unknown-unknown', '-p', 'space-web']);
run('cargo', ['build', '--release', '--target', 'wasm32-unknown-unknown', '-p', 'live-core']);
const out = here('../wasm/');
mkdirSync(out, { recursive: true });
run('wasm-bindgen', ['--target', 'web', '--no-typescript', '--out-dir', out, `${web}/space_web.wasm`]);
copyFileSync(`${release}/live_core.wasm`, `${out}live_core.wasm`);

const apps = ['viewer', 'creator'].map((app) => here(`../../../${app}/static/live/`));
for (const dir of apps) {
	mkdirSync(dir, { recursive: true });
	for (const f of ['space_web.js', 'space_web_bg.wasm', 'live_core.wasm']) copyFileSync(`${out}${f}`, `${dir}${f}`);
}

const kb = (n: number) => `${(n / 1024).toFixed(0)} KB`;
for (const f of ['space_web_bg.wasm', 'space_web.js', 'live_core.wasm']) {
	const raw = statSync(`${out}${f}`).size;
	console.log(`${f}: ${kb(raw)} raw, ${kb(gzipSync(readFileSync(`${out}${f}`), { level: 9 }).length)} gzip`);
}
console.log(`→ ${out} and ${apps.join(" and ")}`);
