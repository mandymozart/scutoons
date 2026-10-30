// The #182 benchmark suite, shared by Node (ts/bench-node.ts) and the browser worker (www/worker.ts). Two axes:
// the language (TypeScript vs deck-core compiled to wasm) and the architecture (patcher = one object + one call per
// node, like Max / Pd / ossia; batch = per dependency level, per kind, struct-of-arrays loops, like an ECS, CHOP
// channels, vvvv gamma / gen~). The TypeScript versions use the same algorithms as deck-core (deck/core/src/math.rs's
// polynomial sine, the same graph shapes), so a difference is the language or the architecture, not the maths.
// Sizes as deck/lab/bench (native): 100 chains = 300 control nodes (deep: 300 levels; wide: 3 levels × 100),
// 16 FM voices, 128-frame blocks, a 1024 FFT, one 330-byte cue envelope.

export const CHAINS = 100;
export const VOICES = 16;
export const FRAMES = 128;
export const SIZE = 1024;
export const RATE = 48_000;
export const ENVELOPE =
	'{"v":1,"seq":4,"at":1761700000250,"from":"c1","to":{"group":"left"},"kind":"cue","p":{"id":"cue-7","actions":[{"say":{"outlet":"left","text":{"de":"Aufstehen","en":"Stand up"},"haptic":true}},{"stage":{"outlet":"left","layer":"l1","show":true}},{"send":{"outlet":"ableton","midi":{"type":"note","ch":1,"n":60,"v":100}}}]}}';

export type Row = { path: string; workload: string; us: number };

const TAU = Math.PI * 2;
const MAGIC = 12_582_912;
const f32 = Math.fround;

// deck/core/src/math.rs in TypeScript (f32 rounding via Math.fround where the trick needs it)
const roundFast = (x: number) => f32(f32(x + MAGIC) - MAGIC);
const wrap01 = (x: number) => x - roundFast(x - 0.5);
export const sinTurns = (x: number): number => {
	const u = x - roundFast(x);
	const a0 = Math.abs(u);
	const q = Math.sign(u) * Math.min(a0, 0.5 - a0);
	const a = TAU * q;
	const a2 = a * a;
	return a * (1 + a2 * (-1 / 6 + a2 * (1 / 120 + a2 * (-1 / 5040 + a2 * (1 / 362_880)))));
};

// ---------------------------------------------------------------- the scene (deck-core's bench_scene, as data)

type Kind = 'lfo' | 'smooth' | 'mul' | 'fm';
type Link = { from: number; scale: number; offset: number };
type SceneNode = { kind: Kind; consts: number[]; links: (Link | null)[] };

/** deck-core's `bench_scene` (deep) / `bench_scene_wide`: node lists with constants and links by index. */
export function scene(chains: number, voices: number, wide: boolean): SceneNode[] {
	const nodes: SceneNode[] = [];
	const at = (k: Kind, consts: number[]) => nodes.push({ kind: k, consts, links: consts.map(() => null) }) - 1;
	const l: number[] = [];
	const s: number[] = [];
	const m: number[] = [];
	for (let i = 0; i < chains; i++) {
		l.push(at('lfo', [0.1 + i * 0.01, 1]));
		s.push(at('smooth', [0, 0.05]));
		m.push(at('mul', [0, 0.5]));
	}
	for (let i = 0; i < chains; i++) {
		nodes[s[i]].links[0] = { from: l[i], scale: 1, offset: 0 };
		nodes[m[i]].links[0] = { from: s[i], scale: 1, offset: 0 };
		if (!wide && i > 0) nodes[l[i]].links[1] = { from: m[i - 1], scale: 1, offset: 1 };
	}
	for (let v = 0; v < voices; v++) {
		const n = at('fm', [0, 2, 1.5, 0.05]);
		nodes[n].links[0] = { from: m[v % chains], scale: 20, offset: 110 * (v + 1) };
	}
	return nodes;
}

function levels(nodes: SceneNode[]): number[] {
	const level = nodes.map(() => -1);
	const visit = (i: number): number => {
		if (level[i] >= 0) return level[i];
		let d = 0;
		for (const k of nodes[i].links) if (k) d = Math.max(d, visit(k.from) + 1);
		return (level[i] = d);
	};
	nodes.forEach((_, i) => visit(i));
	return level;
}

// ---------------------------------------------------------------- architecture 1: the patcher (an object per node)

type TsBlock = {
	control?(i: Float32Array, o: Float32Array, dt: number): void;
	audio?(i: Float32Array, out: Float32Array): void;
};
type Input = { slot: number; scale: number; offset: number } | number;

const lfo = (): TsBlock => {
	let phase = 0;
	return {
		control(i, o, dt) {
			phase = wrap01(phase + i[0] * dt);
			o[0] = i[1] * sinTurns(phase);
		}
	};
};
const mul = (): TsBlock => ({ control: (i, o) => void (o[0] = i[0] * i[1]) });
const smooth = (): TsBlock => {
	let y = 0;
	return {
		control(i, o, dt) {
			const k = i[1] <= 0 ? 1 : 1 - Math.exp(-dt / i[1]);
			y += (i[0] - y) * k;
			o[0] = y;
		}
	};
};
const fm = (): TsBlock => {
	let c = 0;
	let m = 0;
	return {
		audio(i, out) {
			const cs = i[0] / RATE;
			const ms = (i[0] * i[1]) / RATE;
			for (let n = 0; n < out.length; n++) {
				const mo = sinTurns(m) * i[2];
				out[n] = i[3] * sinTurns(c + mo / TAU);
				c = wrap01(c + cs);
				m = wrap01(m + ms);
			}
		}
	};
};
const make: Record<Kind, () => TsBlock> = { lfo, smooth, mul, fm };

export class TsGraph {
	private blocks: TsBlock[];
	private inputs: Input[][];
	private order: number[];
	private values: Float32Array;
	private audio: number[];
	private sin = new Float32Array(8);
	private sout = new Float32Array(8);
	private mix = new Float32Array(FRAMES);

	constructor(nodes: SceneNode[]) {
		this.blocks = nodes.map((n) => make[n.kind]());
		this.inputs = nodes.map((n) => n.consts.map((c, p) => (n.links[p] ? { slot: n.links[p]!.from, ...n.links[p]! } : c)));
		const level = levels(nodes);
		this.order = nodes.map((_, i) => i).sort((a, b) => level[a] - level[b]);
		this.values = new Float32Array(nodes.length);
		this.audio = nodes.flatMap((n, i) => (n.kind === 'fm' ? [i] : []));
	}

	private gather(inputs: Input[]): Float32Array {
		for (let k = 0; k < inputs.length; k++) {
			const x = inputs[k];
			this.sin[k] = typeof x === 'number' ? x : this.values[x.slot] * x.scale + x.offset;
		}
		return this.sin;
	}

	step(dt: number): void {
		for (const i of this.order) {
			const b = this.blocks[i];
			if (!b.control) continue;
			b.control(this.gather(this.inputs[i]), this.sout, dt);
			this.values[i] = this.sout[0];
		}
	}

	render(out: Float32Array): void {
		out.fill(0);
		for (const i of this.audio) {
			this.blocks[i].audio!(this.gather(this.inputs[i]), this.mix);
			for (let k = 0; k < out.length; k++) out[k] += this.mix[k];
		}
	}
}

// ---------------------------------------------------------------- architecture 2: the batch (levels × kinds, SoA)

class Batch {
	src: Int32Array[];
	scale: Float32Array[];
	offset: Float32Array[];
	out: Int32Array;
	s1: Float32Array;
	s2: Float32Array;
	p: Float32Array[];
	y: Float32Array;
	constructor(
		public kind: Kind,
		ids: number[],
		nodes: SceneNode[]
	) {
		const ports = nodes[ids[0]].consts.length;
		const col = <T>(f: (n: SceneNode, p: number) => number, A: new (n: number) => T & { [i: number]: number }) =>
			Array.from({ length: ports }, (_, p) => {
				const a = new A(ids.length);
				ids.forEach((id, i) => (a[i] = f(nodes[id], p)));
				return a;
			});
		// slot 0 is the constant zero; node i writes slot i + 1
		this.src = col((n, p) => (n.links[p] ? n.links[p]!.from + 1 : 0), Int32Array);
		this.scale = col((n, p) => (n.links[p] ? n.links[p]!.scale : 0), Float32Array);
		this.offset = col((n, p) => (n.links[p] ? n.links[p]!.offset : n.consts[p]), Float32Array);
		this.out = Int32Array.from(ids, (i) => i + 1);
		this.s1 = new Float32Array(ids.length);
		this.s2 = new Float32Array(ids.length);
		this.p = Array.from({ length: 4 }, () => new Float32Array(ids.length));
		this.y = new Float32Array(ids.length);
	}
}

export class TsBatch {
	private levels: Batch[][] = [];
	private audio: Batch[] = [];
	private v: Float32Array;

	constructor(nodes: SceneNode[]) {
		const level = levels(nodes);
		const groups = new Map<string, number[]>();
		nodes.forEach((n, i) => {
			const key = n.kind === 'fm' ? 'audio:fm' : `${level[i]}:${n.kind}`;
			groups.set(key, [...(groups.get(key) ?? []), i]);
		});
		for (const [key, ids] of groups) {
			const b = new Batch(nodes[ids[0]].kind, ids, nodes);
			if (key.startsWith('audio')) this.audio.push(b);
			else (this.levels[level[ids[0]]] ??= []).push(b);
		}
		this.v = new Float32Array(nodes.length + 1);
	}

	step(dt: number): void {
		const v = this.v;
		for (const level of this.levels) {
			for (const b of level) {
				const [s0, s1] = b.src;
				const [k0, k1] = b.scale;
				const [o0, o1] = b.offset;
				const n = b.out.length;
				if (b.kind === 'lfo') {
					for (let i = 0; i < n; i++) {
						const ph = wrap01(b.s1[i] + (v[s0[i]] * k0[i] + o0[i]) * dt);
						b.s1[i] = ph;
						v[b.out[i]] = (v[s1[i]] * k1[i] + o1[i]) * sinTurns(ph);
					}
				} else if (b.kind === 'smooth') {
					for (let i = 0; i < n; i++) {
						const t = v[s1[i]] * k1[i] + o1[i];
						const k = t <= 0 ? 1 : 1 - Math.exp(-dt / t);
						b.s1[i] += (v[s0[i]] * k0[i] + o0[i] - b.s1[i]) * k;
						v[b.out[i]] = b.s1[i];
					}
				} else {
					for (let i = 0; i < n; i++) v[b.out[i]] = (v[s0[i]] * k0[i] + o0[i]) * (v[s1[i]] * k1[i] + o1[i]);
				}
			}
		}
	}

	render(out: Float32Array): void {
		out.fill(0);
		const v = this.v;
		for (const b of this.audio) {
			const n = b.out.length;
			const [freq, ratio, index, gain] = b.p;
			for (let p = 0; p < 4; p++) for (let i = 0; i < n; i++) b.p[p][i] = v[b.src[p][i]] * b.scale[p][i] + b.offset[p][i];
			for (let i = 0; i < n; i++) {
				ratio[i] *= freq[i] / RATE;
				freq[i] /= RATE;
			}
			const { s1: c, s2: m, y } = b;
			for (let o = 0; o < out.length; o++) {
				let acc = 0;
				for (let i = 0; i < n; i++) {
					y[i] = gain[i] * sinTurns(c[i] + (sinTurns(m[i]) * index[i]) / TAU);
					c[i] = wrap01(c[i] + freq[i]);
					m[i] = wrap01(m[i] + ratio[i]);
					acc += y[i];
				}
				out[o] += acc;
			}
		}
	}
}

// ---------------------------------------------------------------- the FFT bands (radix-2, in place)

export class TsBands {
	private re = new Float32Array(SIZE);
	private im = new Float32Array(SIZE);
	private hann = new Float32Array(SIZE);
	private cos = new Float32Array(SIZE / 2);
	private sinT = new Float32Array(SIZE / 2);
	private rev = new Uint32Array(SIZE);

	constructor() {
		const bits = Math.log2(SIZE);
		for (let i = 0; i < SIZE; i++) {
			this.hann[i] = 0.5 - 0.5 * Math.cos((TAU * i) / SIZE);
			let r = 0;
			for (let b = 0; b < bits; b++) r |= ((i >> b) & 1) << (bits - 1 - b);
			this.rev[i] = r;
		}
		for (let i = 0; i < SIZE / 2; i++) {
			this.cos[i] = Math.cos((TAU * i) / SIZE);
			this.sinT[i] = -Math.sin((TAU * i) / SIZE);
		}
	}

	analyse(window: Float32Array): [number, number, number, number] {
		const { re, im } = this;
		let sq = 0;
		for (let i = 0; i < SIZE; i++) {
			const x = window[i] * this.hann[i];
			sq += x * x;
			re[this.rev[i]] = x;
			im[this.rev[i]] = 0;
		}
		for (let size = 2; size <= SIZE; size <<= 1) {
			const half = size >> 1;
			const step = SIZE / size;
			for (let start = 0; start < SIZE; start += size) {
				for (let k = 0; k < half; k++) {
					const a = start + k;
					const b = a + half;
					const wr = this.cos[k * step];
					const wi = this.sinT[k * step];
					const tr = re[b] * wr - im[b] * wi;
					const ti = re[b] * wi + im[b] * wr;
					re[b] = re[a] - tr;
					im[b] = im[a] - ti;
					re[a] += tr;
					im[a] += ti;
				}
			}
		}
		const toUnit = (amp: number) => (amp <= 0 ? 0 : Math.min(1, Math.max(0, (20 * Math.log10(amp) + 70) / 60)));
		const binHz = RATE / SIZE;
		const band = (lo: number, hi: number) => {
			const a = Math.max(1, Math.floor(lo / binHz));
			const b = Math.min(SIZE / 2, Math.floor(hi / binHz));
			let peak = 0;
			for (let i = a; i < b; i++) peak = Math.max(peak, Math.sqrt(re[i] * re[i] + im[i] * im[i]));
			return toUnit((peak * 4) / SIZE);
		};
		return [toUnit(Math.sqrt(sq / SIZE) * 2), band(20, 250), band(250, 2000), band(2000, 8000)];
	}
}

export const sineWindow = (): Float32Array => {
	const w = new Float32Array(SIZE);
	for (let i = 0; i < SIZE; i++) w[i] = 0.5 * Math.sin((TAU * 440 * i) / RATE);
	return w;
};

// ---------------------------------------------------------------- timing + the two runs

/** µs per call: warm-up, then 7 runs of `iters` calls, the median. */
export function time(iters: number, f: () => void): number {
	for (let i = 0; i < iters / 10 + 1; i++) f();
	const runs: number[] = [];
	for (let r = 0; r < 7; r++) {
		const t = performance.now();
		for (let i = 0; i < iters; i++) f();
		runs.push(((performance.now() - t) * 1000) / iters);
	}
	runs.sort((a, b) => a - b);
	return runs[3];
}

let sink = 0;
const DEEP = 'control step (300 nodes, deep)';
const WIDE = 'control step (300 nodes, wide)';
const AUDIO = 'audio block (16 FM voices, 128 frames)';
const FFT = 'FFT bands (1024)';
const CODEC = 'codec decode + encode (cue, 330 B)';

export function runTs(path: string): Row[] {
	const rows: Row[] = [];
	const push = (p: string, workload: string, us: number) => rows.push({ path: p, workload, us });
	const out = new Float32Array(FRAMES);
	for (const wide of [false, true]) {
		const nodes = scene(CHAINS, VOICES, wide);
		const g = new TsGraph(nodes);
		const b = new TsBatch(nodes);
		push(path, wide ? WIDE : DEEP, time(20_000, () => g.step(1 / 60)));
		push(`${path} · batch`, wide ? WIDE : DEEP, time(20_000, () => b.step(1 / 60)));
		if (!wide) {
			push(path, AUDIO, time(5_000, () => g.render(out)));
			push(`${path} · batch`, AUDIO, time(5_000, () => b.render(out)));
		}
	}
	const bands = new TsBands();
	const w = sineWindow();
	push(path, FFT, time(5_000, () => (sink += bands.analyse(w)[0])));
	push(path, CODEC, time(50_000, () => (sink += JSON.stringify(JSON.parse(ENVELOPE)).length)));
	return rows;
}

type WasmGraph = {
	step(dt: number): void;
	render(out: Float32Array): void;
	bench_render(blocks: number, frames: number): number;
};

/** The shape of core_wasm.js (wasm-bindgen --target web) that the suite uses. */
export type CoreWasm = {
	Graph: new (n: number, voices: number, wide: boolean) => WasmGraph;
	Batch: new (n: number, voices: number, wide: boolean) => WasmGraph;
	Analyzer: new () => { analyse(w: Float32Array): Float32Array; bench(iters: number): number };
	codec_roundtrip(text: string): string;
	bench_codec(text: string, iters: number): number;
};

/** The wasm paths, per call (crossing the boundary with a Float32Array, as a worker would). */
export function runWasm(path: string, w: CoreWasm): Row[] {
	const rows: Row[] = [];
	const push = (p: string, workload: string, us: number) => rows.push({ path: p, workload, us });
	const out = new Float32Array(FRAMES);
	for (const wide of [false, true]) {
		const g = new w.Graph(CHAINS, VOICES, wide);
		const b = new w.Batch(CHAINS, VOICES, wide);
		push(path, wide ? WIDE : DEEP, time(20_000, () => g.step(1 / 60)));
		push(`${path} · batch`, wide ? WIDE : DEEP, time(20_000, () => b.step(1 / 60)));
		if (!wide) {
			push(path, AUDIO, time(5_000, () => g.render(out)));
			push(`${path} · batch`, AUDIO, time(5_000, () => b.render(out)));
		}
	}
	const a = new w.Analyzer();
	const win = sineWindow();
	push(path, FFT, time(5_000, () => (sink += a.analyse(win)[0])));
	push(path, CODEC, time(50_000, () => (sink += w.codec_roundtrip(ENVELOPE).length)));
	push(`${path} (loop inside)`, CODEC, time(20, () => (sink += w.bench_codec(ENVELOPE, 1000))) / 1000);
	return rows;
}

export const keepAlive = (): number => sink;
