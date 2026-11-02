// P0c (#309): FM16 in V8 – the lab engine (the stage's voice batches lifted to wasm32 + simd128) against live-core's
// BatchGraph (the same 16 FM voices), 128 frames per block, plus the files' sizes (raw, gzip).
//   node bench.mjs <lab.wasm> <live_core.wasm> [<live_core without simd.wasm>]   → one JSON line on stdout
import { readFileSync } from 'node:fs';
import { gzipSync } from 'node:zlib';

const [labPath, corePath, coreBasePath] = process.argv.slice(2);
const BLOCK = 128;
const BLOCKS = 400; // per sample of the median: 400 blocks = 1.07 s of audio
const SAMPLES = 25;

const scene = Array.from({ length: 16 }, (_, i) => `[[node]]\nid = "v${i}"\nkind = "fm"\nfreq = ${110 + 13 * i}.0\nratio = 2.0\nindex = 1.5\ngain = 0.05\n`).join('');

function median(a) {
	const s = [...a].sort((x, y) => x - y);
	return s[s.length >> 1];
}

function time(run) {
	for (let i = 0; i < 1500; i++) run(); // warm-up: tier-up in V8
	const per = [];
	for (let s = 0; s < SAMPLES; s++) {
		const t = performance.now();
		for (let i = 0; i < BLOCKS; i++) run();
		per.push(((performance.now() - t) * 1000) / BLOCKS);
	}
	return { median_us: median(per), min_us: Math.min(...per), max_us: Math.max(...per) };
}

async function lab(full) {
	const bytes = readFileSync(labPath);
	const { instance } = await WebAssembly.instantiate(bytes, {});
	const x = instance.exports;
	x.lab_bench_init(0.0);
	return time(() => x.lab_bench_block(BLOCK, full));
}

async function core(path) {
	const bytes = readFileSync(path);
	const { instance } = await WebAssembly.instantiate(bytes, {});
	const x = instance.exports;
	x.lc_init(48000);
	const text = new TextEncoder().encode(scene);
	const ptr = x.lc_inbox(text.length);
	new Uint8Array(x.memory.buffer, ptr, text.length).set(text);
	if (x.lc_load(text.length) < 0) throw new Error('live-core refused the scene');
	return time(() => {
		x.lc_step(BLOCK / 48000);
		x.lc_render(BLOCK);
	});
}

const out = {
	block: BLOCK,
	budget_us: (BLOCK / 48000) * 1e6,
	lab_voices: await lab(0),
	lab_with_master: await lab(1),
	live_core: await core(corePath),
	size: {
		lab: { raw: readFileSync(labPath).length, gz: gzipSync(readFileSync(labPath), { level: 9 }).length },
		live_core: { raw: readFileSync(corePath).length, gz: gzipSync(readFileSync(corePath), { level: 9 }).length }
	}
};
if (coreBasePath) {
	out.live_core_nosimd = await core(coreBasePath);
	out.size.live_core_nosimd = { raw: readFileSync(coreBasePath).length, gz: gzipSync(readFileSync(coreBasePath), { level: 9 }).length };
}
console.log(JSON.stringify(out));
