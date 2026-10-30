// `node --experimental-transform-types deck/lab/ts/bench-formats.ts [runs]` – the #184 exchange formats in Node
// (V8, the engine of Chrome's workers), `runs` times (default 5); medians + spread into results/formats-node.json.
// Needs www/pkg (core-wasm via wasm-bindgen) and deck/target/…/live_core.wasm (deck/lab/README.md).
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { WasmCore } from '../../../shared/live-core/src/engine.ts';
import { checkPostcard, formatsSink, runFormats, summarise, table, type FormatsWasm } from './formats.ts';

const lab = fileURLToPath(new URL('..', import.meta.url));
const pkg = (await import(new URL('../www/pkg/core_wasm.js', import.meta.url).href)) as FormatsWasm & { initSync(o: { module: BufferSource }): unknown };
pkg.initSync({ module: readFileSync(`${lab}/www/pkg/core_wasm_bg.wasm`) });
const core = await WasmCore.instantiate(readFileSync(`${lab}/../target/wasm32-unknown-unknown/release/live_core.wasm`), 48_000);
core.load(pkg.fmt_scene());
console.log('postcard TS → Rust:', checkPostcard(pkg).slice(0, 160), '…');

const n = Number(process.argv[2] ?? 5);
const runs = [];
for (let i = 0; i < n; i++) {
	runs.push(runFormats('Node 22', pkg, core));
	console.error(`run ${i + 1}/${n}`);
}
const rows = summarise(runs);
console.log(table(rows));
mkdirSync(`${lab}/results`, { recursive: true });
writeFileSync(`${lab}/results/formats-node.json`, JSON.stringify(rows, null, 2));
if (formatsSink() === 42.4242) console.log('');
