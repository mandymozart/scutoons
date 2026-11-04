// node.ts – the #306 suite's browser runtimes in Node's V8 (worker threads): the PC's quick V8 numbers without a
// browser. `node --experimental-strip-types deck/module-bench/www/node.ts [--label pc] [--blocks 2000]`
// → deck/module-bench/results/<label>-node.json + .md (MODULE-BENCH.md).
import { Worker } from 'node:worker_threads';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import os from 'node:os';
import { runAll, table, type WorkerLike } from './bench.ts';

const arg = (name: string, fallback: string) => {
	const i = process.argv.indexOf(`--${name}`);
	return i > 0 ? process.argv[i + 1] : fallback;
};
const label = `${arg('label', 'pc')}-node`;
const dist = new URL('./dist/', import.meta.url);
const results = new URL('../results/', import.meta.url);

const spawn = (): WorkerLike => {
	const w = new Worker(new URL('./worker.ts', import.meta.url));
	return { post: (m) => w.postMessage(m), on: (f) => w.once('message', f), terminate: () => w.terminate() };
};

const runtimes = await runAll(spawn, { blocks: Number(arg('blocks', '2000')) }, (s) => console.log(s));
const all = {
	device: {
		label,
		os: `${os.platform()} ${os.release()} ${os.arch()}`,
		cpu: os.cpus()[0]?.model ?? '',
		cores: os.cpus().length,
		user_agent: `node ${process.version} (V8 ${process.versions.v8})`,
		when: new Date().toISOString().slice(0, 16).replace('T', ' ') + ' UTC',
		notes: []
	},
	runtimes,
	sizes: JSON.parse(await readFile(new URL('sizes.json', dist), 'utf8'))
};
const md = await table(new Uint8Array(await readFile(new URL('bench-web.wasm', dist))), [all]);
console.log(md);
await mkdir(results, { recursive: true });
await writeFile(new URL(`${label}.json`, results), JSON.stringify(all, null, 2));
await writeFile(new URL(`${label}.md`, results), md);
console.log(`→ results/${label}.json`);
