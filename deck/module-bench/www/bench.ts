// bench.ts – the #306 module benchmark's platform side (agents/research/MODULE-BENCH.md), for a browser worker and Node
// alike. The suite is Rust (deck/module-bench, built into dist/bench-web*.wasm); this file only gives it the platform's
// WebAssembly behind the `bench.*` imports – compile, instantiate, call an export, copy bytes, the gas global, the
// clock – the shape of the `web` backend #290 would ship (no logic here: the host's rules stay in deck-module-host).
// It also runs the runtimes one after another, each in a fresh worker, and the watchdog: the plain browser runtime
// cannot stop a runaway in-thread, so the parent terminates the worker after the budget and times a new one.

/** deck-module-host's `Export::ALL`, in order (the index the suite passes to `call`) */
const EXPORTS = ['dm_abi', 'dm_io', 'dm_layout', 'dm_new', 'dm_process', 'dm_control', 'dm_asset_buf'];
const KINDS = ['fm16', 'walk', 'analog', 'stress'];

/** bytes on a plain ArrayBuffer (what WebAssembly takes) */
export type Bytes = Uint8Array<ArrayBuffer>;
type Inst = { fns: (Function | undefined)[]; memory: WebAssembly.Memory; gas?: WebAssembly.Global };
type Exports = Record<string, any>;

/** the bench wasm with its imports bound */
async function load(wasm: Bytes, mark: () => void = () => {}): Promise<{ ex: Exports; u8: () => Uint8Array }> {
	let mem: WebAssembly.Memory | undefined;
	const u8 = () => new Uint8Array(mem!.buffer);
	const mods: WebAssembly.Module[] = [];
	const insts: (Inst | undefined)[] = [];
	let lastError = '';
	const put = (s: string, ptr: number, cap: number) => {
		const b = new TextEncoder().encode(s).subarray(0, cap);
		u8().set(b, ptr);
		return b.length;
	};
	const bench = {
		now_us: () => performance.now() * 1000,
		mark,
		compile: (ptr: number, len: number) => {
			try {
				mods.push(new WebAssembly.Module(u8().slice(ptr, ptr + len)));
				return mods.length - 1;
			} catch (e) {
				lastError = String(e);
				return -1;
			}
		},
		module_info: (id: number, out: number, cap: number) => {
			const m = mods[id];
			const info = { imports: WebAssembly.Module.imports(m).map((i) => [i.module, i.name]), exports: WebAssembly.Module.exports(m).map((e) => e.name) };
			return put(JSON.stringify(info), out, cap);
		},
		instantiate: (id: number, gas: number) => {
			try {
				let memory: WebAssembly.Memory | undefined;
				const deck_log = (_level: number, ptr: number, len: number) => console.log(new TextDecoder().decode(new Uint8Array(memory!.buffer, ptr, len)));
				const i = new WebAssembly.Instance(mods[id], { env: { deck_log } });
				const e = i.exports as Exports;
				memory = e.memory as WebAssembly.Memory;
				insts.push({ fns: EXPORTS.map((n) => e[n] as Function | undefined), memory, gas: gas ? (e.gas_left as WebAssembly.Global) : undefined });
				return insts.length - 1;
			} catch (e) {
				lastError = String(e);
				return -1;
			}
		},
		drop_instance: (i: number) => {
			insts[i] = undefined;
		},
		call: (i: number, idx: number, a0: number, a1: number) => {
			const x = insts[i]!;
			try {
				const r = x.fns[idx]!(a0, a1);
				return typeof r === 'number' ? r >>> 0 : 0;
			} catch (e) {
				// out of gas: the global below 0 (wasm-instrument's mark is u64::MAX, the loop counter's -1)
				if (x.gas && x.gas.value < 0n) return -2;
				lastError = String(e);
				return -1;
			}
		},
		mem_size: (i: number) => insts[i]!.memory.buffer.byteLength,
		mem_write: (i: number, off: number, src: number, len: number) => new Uint8Array(insts[i]!.memory.buffer, off, len).set(u8().subarray(src, src + len)),
		mem_read: (i: number, off: number, dst: number, len: number) => u8().set(new Uint8Array(insts[i]!.memory.buffer, off, len), dst),
		set_gas: (i: number, g: number) => {
			insts[i]!.gas!.value = BigInt(Math.floor(g));
		},
		gas_left: (i: number) => Number(insts[i]!.gas!.value),
		last_error: (out: number, cap: number) => put(lastError, out, cap)
	};
	const { instance } = await WebAssembly.instantiate(wasm as BufferSource, { bench });
	const ex = instance.exports as Exports;
	mem = ex.memory as WebAssembly.Memory;
	return { ex, u8 };
}

function send(ex: Exports, u8: () => Uint8Array, bytes: Uint8Array): number {
	const p = ex.bench_alloc(bytes.length);
	u8().set(bytes, p);
	return p;
}

function answer(ex: Exports, u8: () => Uint8Array, n: number): string {
	const p = ex.bench_out();
	return new TextDecoder().decode(u8().slice(p, p + n));
}

/** what a worker needs: the bench wasm and the four modules (plain or gas-metered) */
export type Files = { wasm: Bytes; modules: Record<string, Bytes> };

/** in a worker: the suite on one runtime → its RuntimeResult. `web-runaway` never returns (the watchdog's case). */
export async function suite(files: Files, runtime: string, cfg: object, mark?: () => void): Promise<any> {
	const { ex, u8 } = await load(files.wasm, mark);
	KINDS.forEach((k, i) => ex.bench_set_module(i, send(ex, u8, files.modules[k]), files.modules[k].length));
	const rt = new TextEncoder().encode(runtime);
	const c = new TextEncoder().encode(JSON.stringify(cfg));
	return JSON.parse(answer(ex, u8, ex.bench_run(send(ex, u8, rt), rt.length, send(ex, u8, c), c.length)));
}

/** the Markdown table + gates of a list of Results (the Rust report, run here) */
export async function table(wasm: Bytes, all: object[]): Promise<string> {
	const { ex, u8 } = await load(wasm);
	const b = new TextEncoder().encode(JSON.stringify(all));
	return answer(ex, u8, ex.bench_table(send(ex, u8, b), b.length));
}

/** the files of one runtime, by name in dist/ */
export function filesOf(runtime: string): { wasm: string; modules: Record<string, string> } {
	const wasm = runtime === 'web-wasmi' ? 'bench-web-wasmi.wasm' : 'bench-web.wasm';
	const suffix = runtime === 'web-gas' ? '.gas.wasm' : runtime === 'web-loops' ? '.loops.wasm' : '.wasm';
	return { wasm, modules: Object.fromEntries(KINDS.map((k) => [k, k + suffix])) };
}

/** a worker as both platforms have one: post, one message handler, terminate */
export type WorkerLike = { post(m: unknown): void; on(f: (m: any) => void): void; terminate(): Promise<unknown> | void };

const epochMs = () => performance.timeOrigin + performance.now();

function once(w: WorkerLike): Promise<any> {
	return new Promise((resolve) => w.on(resolve));
}

/** Every runtime in a fresh worker, then the watchdog on the plain runtime → the RuntimeResults. */
export async function runAll(spawn: () => WorkerLike, cfg: object, log: (s: string) => void, runtimes = ['web', 'web-gas', 'web-loops', 'web-wasmi']): Promise<any[]> {
	const out: any[] = [];
	for (const rt of runtimes) {
		log(`${rt} …`);
		const w = spawn();
		w.post({ runtime: rt, cfg: { ...cfg, runaway: rt !== 'web' } });
		const r = await once(w);
		await w.terminate();
		out.push(r.result ?? { runtime: rt, place: 'worker', error: r.error });
		log(`${rt}: ${r.error ?? 'done'}`);
	}
	const web = out.find((r) => r.runtime === 'web');
	if (web) {
		log('watchdog …');
		web.runaway = await watchdog(spawn);
	}
	return out;
}

/** The plain runtime's runaway: the worker calls the looping module; after the strict budget (2 ms) the parent
 *  terminates it and starts a new worker that loads the bench and compiles the module again (the restart). */
async function watchdog(spawn: () => WorkerLike): Promise<object> {
	const budgetMs = 2;
	const w = spawn();
	w.post({ runtime: 'web-runaway', cfg: {} });
	const calling = await once(w); // { calling: epoch ms }: the suite's `mark`, right before the module's call
	const until = calling.calling + budgetMs;
	while (epochMs() < until) await new Promise((r) => setTimeout(r, 0));
	const stopped = epochMs();
	await w.terminate();
	const terminated = epochMs();
	const fresh = spawn();
	fresh.post({ runtime: 'ready', cfg: {} });
	await once(fresh);
	const ready = epochMs();
	await fresh.terminate();
	return {
		how: 'watchdog (the parent terminates the worker)',
		budget_ms: budgetMs,
		stopped_after_us: (terminated - calling.calling) * 1000,
		over_budget_us: (terminated - until) * 1000,
		restart_us: (ready - terminated) * 1000,
		fault: `terminate() took ${((terminated - stopped) * 1000).toFixed(0)} µs; the restart is a new worker + the bench wasm + the module compiled`
	};
}

/** the worker's side: one message, one answer */
export async function serve(msg: { runtime: string; cfg: object }, read: (name: string) => Promise<Bytes>, reply: (m: unknown) => void) {
	try {
		const names = filesOf(msg.runtime === 'web-runaway' || msg.runtime === 'ready' ? 'web' : msg.runtime);
		const files: Files = { wasm: await read(names.wasm), modules: {} };
		for (const [k, n] of Object.entries(names.modules)) files.modules[k] = await read(n);
		if (msg.runtime === 'ready') {
			// the restart: the bench loaded and the module compiled, as a worker taking over would
			await WebAssembly.compile(files.modules.stress);
			await load(files.wasm);
			reply({ ready: true });
			return;
		}
		reply({ result: await suite(files, msg.runtime, msg.cfg, () => reply({ calling: epochMs() })) });
	} catch (e) {
		reply({ error: String(e) });
	}
}
