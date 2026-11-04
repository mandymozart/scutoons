// `node --experimental-transform-types deck/lab/ts/build.ts` – builds the lab pages into www/dist:
// - the #182 page: its TypeScript type-stripped file by file (typescript's transpileModule, no bundler), wasm-bindgen's
//   www/pkg and www/pkg-simd copied next to it;
// - the #184 / #197 pages (formats.html, demo.html, deck.html – over @scutoons/live-deck since #198): bundled with rolldown (vite 8's bundler, from the repo's node_modules) –
//   they import shared/live-core; the worker and the worklet are their own entries, live_core.wasm is copied from
//   deck/target (`npm run build:wasm -w shared/live-core`).
import { readFileSync, writeFileSync, mkdirSync, cpSync, existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const lab = fileURLToPath(new URL('..', import.meta.url));
const require = createRequire(import.meta.url);
const ts = require('typescript') as typeof import('typescript');

// LAB_DIST: another folder (a check build while the served www/dist stays as it is)
const dist = process.env.LAB_DIST ?? `${lab}/www/dist`;
mkdirSync(dist, { recursive: true });
for (const [src, name] of [
	['ts/suite.ts', 'suite.js'],
	['www/worker.ts', 'worker.js'],
	['www/main.ts', 'main.js']
]) {
	const { outputText } = ts.transpileModule(readFileSync(`${lab}/${src}`, 'utf8'), {
		compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 }
	});
	writeFileSync(`${dist}/${name}`, outputText.replace("from './suite.ts'", "from './suite.js'"));
}
for (const pkg of ['pkg', 'pkg-simd']) if (existsSync(`${lab}/www/${pkg}`)) cpSync(`${lab}/www/${pkg}`, `${dist}/${pkg}`, { recursive: true });

const { rolldown } = (await import('rolldown')) as typeof import('rolldown');
const liveCore = `${lab}/../../shared/live-core/src`;
const bundle = await rolldown({
	input: {
		'formats-main': `${lab}/www/formats-main.ts`,
		'formats-worker': `${lab}/www/formats-worker.ts`,
		demo: `${lab}/www/demo.ts`,
		deck: `${lab}/www/deck.ts`,
		'live-worker': `${liveCore}/worker.ts`,
		'live-worklet': `${liveCore}/worklet.ts`
	},
	platform: 'browser'
});
await bundle.write({ dir: dist, format: 'esm', entryFileNames: '[name].js', chunkFileNames: 'chunks/[name].js' });
const wasm = `${lab}/../target/wasm32-unknown-unknown/release/live_core.wasm`;
if (existsSync(wasm)) cpSync(wasm, `${dist}/live_core.wasm`);
else console.warn('live_core.wasm is not built – npm run build:wasm -w shared/live-core');
// deck.html's `?space=1` (#198): the being in 3D – deck/space-web's wasm (`npm run build:wasm -w shared/live-deck`)
const spaceWeb = `${lab}/../../shared/live-deck/wasm`;
if (existsSync(`${spaceWeb}/space_web_bg.wasm`)) {
	mkdirSync(`${dist}/space-web`, { recursive: true });
	// #366: relay-web (the relay client) beside it
	for (const f of ['space_web.js', 'space_web_bg.wasm', 'relay_web.js', 'relay_web_bg.wasm']) if (existsSync(`${spaceWeb}/${f}`)) cpSync(`${spaceWeb}/${f}`, `${dist}/space-web/${f}`);
} else console.warn('space-web is not built – npm run build:wasm -w shared/live-deck');
// deck.html (#197) wears the shared tokens
cpSync(`${lab}/../../shared/tokens/tokens.css`, `${dist}/tokens.css`);
console.log(`built ${dist}`);
