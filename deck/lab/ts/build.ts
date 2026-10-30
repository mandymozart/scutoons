// `node --experimental-transform-types deck/lab/ts/build.ts` – type-strips the lab page's TypeScript into www/dist
// (typescript's transpileModule from the repo's node_modules; no bundler) and copies wasm-bindgen's www/pkg and
// www/pkg-simd there.
import { readFileSync, writeFileSync, mkdirSync, cpSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const lab = fileURLToPath(new URL('..', import.meta.url));
const require = createRequire(import.meta.url);
const ts = require('typescript') as typeof import('typescript');

const dist = `${lab}/www/dist`;
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
for (const pkg of ['pkg', 'pkg-simd']) cpSync(`${lab}/www/${pkg}`, `${dist}/${pkg}`, { recursive: true });
console.log(`built ${dist}`);
