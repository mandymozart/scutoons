// `node --experimental-transform-types deck/lab/ts/bench-node.ts` – the suite in Node (V8, the engine of Chrome's
// workers): TypeScript and the two wasm builds (plain, simd128). Needs www/pkg + www/pkg-simd (deck/lab/README.md). Writes results/node.json.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { runTs, runWasm, keepAlive, type CoreWasm } from './suite.ts';

const lab = fileURLToPath(new URL('..', import.meta.url));
type Pkg = CoreWasm & { initSync(o: { module: BufferSource }): unknown };
const load = async (dir: string): Promise<Pkg> => {
	const pkg = (await import(new URL(`../www/${dir}/core_wasm.js`, import.meta.url).href)) as Pkg;
	pkg.initSync({ module: readFileSync(`${lab}/www/${dir}/core_wasm_bg.wasm`) });
	return pkg;
};

const rows = [
	...runTs('TypeScript (Node)'),
	...runWasm('wasm (Node)', await load('pkg')),
	...runWasm('wasm simd128 (Node)', await load('pkg-simd'))
];
console.log('| path | workload | µs |\n|---|---|---|');
for (const r of rows) console.log(`| ${r.path} | ${r.workload} | ${r.us.toFixed(3)} |`);
mkdirSync(`${lab}/results`, { recursive: true });
writeFileSync(`${lab}/results/node.json`, JSON.stringify(rows, null, 2));
if (keepAlive() === 42.4242) console.log('');
