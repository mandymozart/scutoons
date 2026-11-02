// The browser's module host (#280): runs a rig module – wasm built against deck/module (the SDK, ABI v1:
// deck/module/README.md) – natively in the browser's engine, the same `.wasm` the deck's wasmtime host runs
// (deck/stage/src/module_host.rs). No wasm-bindgen: the descriptor comes out of the `deck.module` custom section
// without instantiating, `Io` is read and written through the generated offsets (module-abi.ts), one import
// (`env.deck_log`, rate-limited). A trap silences the instance – never the page. Run it in the worker, off the main
// thread: a browser has no epoch budget, a runaway loop is the worker's to terminate (the engine's watchdog).
import {
	ABI_VERSION,
	EVENT_NAME_LEN,
	MAX_AUDIO_PORTS,
	MAX_BLOCK,
	MAX_CONTROL_IN,
	MAX_EVENTS,
	MAX_PARAMS,
	MAX_VARS,
	OFFSETS,
	PROP_KEY_LEN,
	SECTION,
	STR_LEN,
	VAR_KEY_LEN
} from './module-abi.ts';

export type PortKind = 'audio' | 'control' | 'video' | 'event';
export interface ParamDesc {
	name: string;
	min: number;
	max: number;
	default: number;
}
export interface PortDesc {
	name: string;
	dir: 'in' | 'out';
	kind: PortKind;
}
/** What a module declares, read before anything runs. */
export interface Descriptor {
	abi: number;
	name: string;
	version: string;
	params: ParamDesc[];
	ports: PortDesc[];
}
/** One event a module emitted (deck-events' model: a name, beats, up to four numeric props). */
export interface ModuleEvent {
	name: string;
	start: number;
	len: number;
	port: number;
	props: Record<string, number>;
}
/** The tempo map's anchor (deck-core's TempoMap): frame `frame` is beat `beat`, from there `bpm`. */
export interface Tempo {
	frame: number;
	beat: number;
	bpm: number;
	rate: number;
	running: boolean;
	perBar: number;
}

const ident = (s: unknown): s is string => typeof s === 'string' && /^[a-z][a-z0-9_]{0,23}$/.test(s);

/** Parses and checks a descriptor (the Rust `Descriptor::parse`): the ABI version, the names, the ranges, the counts. */
export function parseDescriptor(json: string): Descriptor {
	const d = JSON.parse(json) as Descriptor;
	if (d.abi !== ABI_VERSION) throw new Error(`module '${d.name}' speaks ABI ${d.abi}, this host speaks ${ABI_VERSION}`);
	if (!ident(d.name)) throw new Error(`module name '${d.name}' is not a lowercase identifier`);
	if (!Array.isArray(d.params) || !Array.isArray(d.ports)) throw new Error('descriptor: params and ports are lists');
	if (d.params.length > MAX_PARAMS) throw new Error(`module '${d.name}' has ${d.params.length} parameters, at most ${MAX_PARAMS}`);
	const names = new Set<string>();
	for (const p of d.params) {
		if (!ident(p.name)) throw new Error(`parameter name '${p.name}' is not a lowercase identifier`);
		if (!(Number.isFinite(p.min) && Number.isFinite(p.max) && p.min < p.max && p.default >= p.min && p.default <= p.max)) {
			throw new Error(`parameter '${p.name}': needs min < max and a default inside`);
		}
		names.add(p.name);
	}
	for (const p of d.ports) {
		if (!ident(p.name)) throw new Error(`port name '${p.name}' is not a lowercase identifier`);
		if (names.has(p.name)) throw new Error(`module '${d.name}': the name '${p.name}' is used twice`);
		names.add(p.name);
	}
	const count = (dir: string, kind: string) => d.ports.filter((p) => p.dir === dir && p.kind === kind).length;
	for (const [dir, kind, max] of [['in', 'audio', MAX_AUDIO_PORTS], ['out', 'audio', MAX_AUDIO_PORTS], ['in', 'control', MAX_CONTROL_IN]] as const) {
		if (count(dir, kind) > max) throw new Error(`module '${d.name}' has more than ${max} ${kind} ports ${dir}`);
	}
	return d;
}

/** The descriptor out of a compiled module's `deck.module` section – nothing runs. */
export function readDescriptor(module: WebAssembly.Module): Descriptor {
	const [section] = WebAssembly.Module.customSections(module, SECTION);
	if (!section) throw new Error('no `deck.module` section – the wasm was not built with deck-module');
	return parseDescriptor(new TextDecoder().decode(section));
}

/** The module's `log!` budget: a burst, then a steady rate (the Rust host's `LogLimiter`). */
export class LogLimiter {
	private tokens = 10;
	private last: number | null = null;
	dropped = 0;
	allow(nowMs: number): boolean {
		if (this.last !== null) this.tokens = Math.min(10, this.tokens + ((nowMs - this.last) / 1000) * 10);
		this.last = nowMs;
		if (this.tokens >= 1) {
			this.tokens -= 1;
			return true;
		}
		this.dropped++;
		return false;
	}
}

interface Exports {
	memory: WebAssembly.Memory;
	dm_abi(): number;
	dm_io(): number;
	dm_new(): void;
	dm_set_param(index: number, value: number): void;
	dm_process(frames: number): void;
	dm_control(frames: number): void;
}

export interface InstanceOptions {
	/** the module's seed – the same seed plays the same on every deck and page (a bigint or a safe integer) */
	seed?: number | bigint;
	/** `log!` lines that pass the rate limit */
	onLog?: (level: 'info' | 'warn' | 'error', text: string) => void;
	/** a clock for the log limiter (tests) */
	now?: () => number;
}

/** One running module. */
export class ModuleInstance {
	readonly desc: Descriptor;
	/** why this instance is silent (a trap), if it is */
	silenced: string | null = null;
	private readonly x: Exports;
	private readonly io: number;
	private view: DataView;
	private bytes: Uint8Array;
	private buffer: ArrayBuffer;

	private constructor(desc: Descriptor, x: Exports) {
		this.desc = desc;
		this.x = x;
		this.io = x.dm_io();
		this.buffer = x.memory.buffer;
		this.view = new DataView(this.buffer);
		this.bytes = new Uint8Array(this.buffer);
	}

	/** Compiles (or takes a compiled module), checks the descriptor and the exports, builds the module. */
	static async create(wasm: WebAssembly.Module | BufferSource, rate: number, opts: InstanceOptions = {}): Promise<ModuleInstance> {
		const module = wasm instanceof WebAssembly.Module ? wasm : await WebAssembly.compile(wasm);
		return ModuleInstance.fromModule(module, rate, opts);
	}

	static async fromModule(module: WebAssembly.Module, rate: number, opts: InstanceOptions = {}): Promise<ModuleInstance> {
		const desc = readDescriptor(module);
		for (const imp of WebAssembly.Module.imports(module)) {
			if (imp.module !== 'env' || imp.name !== 'deck_log') throw new Error(`module '${desc.name}' imports ${imp.module}.${imp.name} – only env.deck_log is allowed`);
		}
		const limiter = new LogLimiter();
		const now = opts.now ?? (() => performance.now());
		let memory: WebAssembly.Memory | null = null;
		const instance = await WebAssembly.instantiate(module, {
			env: {
				deck_log(level: number, ptr: number, len: number) {
					if (!memory || !limiter.allow(now())) return;
					const text = new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, len));
					opts.onLog?.(level === 0 ? 'info' : level === 1 ? 'warn' : 'error', text);
				}
			}
		});
		const x = instance.exports as unknown as Exports;
		memory = x.memory;
		for (const name of ['memory', 'dm_abi', 'dm_io', 'dm_new', 'dm_set_param', 'dm_process', 'dm_control']) {
			if (!(name in x)) throw new Error(`module '${desc.name}' does not export \`${name}\``);
		}
		if (x.dm_abi() !== ABI_VERSION) throw new Error(`module '${desc.name}' speaks ABI ${x.dm_abi()}, this host speaks ${ABI_VERSION}`);
		const me = new ModuleInstance(desc, x);
		if (me.io + OFFSETS.size > x.memory.buffer.byteLength) throw new Error(`module '${desc.name}': its Io does not lie in its memory`);
		me.setTempo({ frame: 0, beat: 0, bpm: 120, rate, running: true, perBar: 4 });
		me.f64(OFFSETS.rate, rate);
		me.view.setBigUint64(me.io + OFFSETS.seed, BigInt.asUintN(64, BigInt(opts.seed ?? 1)), true);
		try {
			x.dm_new();
		} catch (e) {
			throw new Error(`module '${desc.name}': dm_new: ${(e as Error).message}`);
		}
		return me;
	}

	// a grown memory detaches the views: re-made when the buffer changed
	private fresh(): void {
		if (this.x.memory.buffer !== this.buffer) {
			this.buffer = this.x.memory.buffer;
			this.view = new DataView(this.buffer);
			this.bytes = new Uint8Array(this.buffer);
		}
	}

	private f64(at: number, v: number): void {
		this.fresh();
		this.view.setFloat64(this.io + at, v, true);
	}

	private u32(at: number, v: number): void {
		this.fresh();
		this.view.setUint32(this.io + at, v, true);
	}

	private f32(at: number, v: number): void {
		this.fresh();
		this.view.setFloat32(this.io + at, v, true);
	}

	private str(at: number, len: number, s: string): void {
		this.fresh();
		const field = this.bytes.subarray(this.io + at, this.io + at + len);
		field.fill(0);
		// cut at a character boundary, like the Rust side (`put_str`)
		const enc = new TextEncoder();
		let used = 0;
		for (const ch of s) {
			const bytes = enc.encode(ch);
			if (used + bytes.length > len) break;
			field.set(bytes, used);
			used += bytes.length;
		}
	}

	setTempo(t: Tempo): void {
		this.f64(OFFSETS.tempoFrame, t.frame);
		this.f64(OFFSETS.tempoBeat, t.beat);
		this.f64(OFFSETS.bpm, t.bpm);
		this.f64(OFFSETS.rate, t.rate);
		this.u32(OFFSETS.running, t.running ? 1 : 0);
		this.u32(OFFSETS.perBar, t.perBar);
	}

	/** the character the module plays for (empty = none) */
	setCharacter(id: string, persona: string, mood: string): void {
		this.str(OFFSETS.characterId, STR_LEN, id);
		this.str(OFFSETS.characterPersona, STR_LEN, persona);
		this.str(OFFSETS.characterMood, STR_LEN, mood);
	}

	/** scene and world variables, key → number (at most 32) */
	setVars(vars: Record<string, number>): void {
		const entries = Object.entries(vars).slice(0, MAX_VARS);
		entries.forEach(([k, v], i) => {
			this.str(OFFSETS.varKeys + i * VAR_KEY_LEN, VAR_KEY_LEN, k);
			this.f32(OFFSETS.varValues + i * 4, v);
		});
		this.u32(OFFSETS.varsLen, entries.length);
	}

	/** parameter `index` (declaration order), clamped by the module */
	setParam(index: number, value: number): void {
		if (this.silenced) return;
		this.guard(() => this.x.dm_set_param(index, value), 'dm_set_param');
	}

	setControlIn(port: number, value: number): void {
		if (port < MAX_CONTROL_IN) this.f32(OFFSETS.controlIn + port * 4, value);
	}

	setAudioIn(port: number, l: Float32Array, r: Float32Array): void {
		if (port >= MAX_AUDIO_PORTS) return;
		this.fresh();
		const n = Math.min(l.length, r.length, MAX_BLOCK);
		const at = this.io + OFFSETS.audioIn + port * 2 * MAX_BLOCK * 4;
		new Float32Array(this.buffer, at, n).set(l.subarray(0, n));
		new Float32Array(this.buffer, at + MAX_BLOCK * 4, n).set(r.subarray(0, n));
	}

	/** the audio output `port` of the last `process` into `l` / `r` (their lengths = the frames asked) */
	readAudioOut(port: number, l: Float32Array, r: Float32Array): void {
		const n = Math.min(l.length, r.length, MAX_BLOCK);
		if (port >= MAX_AUDIO_PORTS) {
			l.fill(0);
			r.fill(0);
			return;
		}
		this.fresh();
		const at = this.io + OFFSETS.audioOut + port * 2 * MAX_BLOCK * 4;
		l.set(new Float32Array(this.buffer, at, n));
		r.set(new Float32Array(this.buffer, at + MAX_BLOCK * 4, n));
	}

	/** One audio call of `frames` (≤ 1024) frames starting at audio frame `frame`. False when the instance is silent. */
	process(frame: number, frames: number): boolean {
		return this.call(frame, frames, 'dm_process');
	}

	control(frame: number, frames: number): boolean {
		return this.call(frame, frames, 'dm_control');
	}

	private call(frame: number, frames: number, fn: 'dm_process' | 'dm_control'): boolean {
		if (this.silenced) return false;
		this.f64(OFFSETS.frame, frame);
		return this.guard(() => this.x[fn](Math.min(frames, MAX_BLOCK)), fn);
	}

	// a trap (a panic is one) silences the instance: the message is kept, outputs stay silent
	private guard(run: () => void, what: string): boolean {
		try {
			run();
			return true;
		} catch (e) {
			this.silenced = `${what}: ${(e as Error).message}`;
			return false;
		}
	}

	/** the events the last call emitted */
	events(): ModuleEvent[] {
		this.fresh();
		const n = Math.min(this.view.getUint32(this.io + OFFSETS.eventsLen, true), MAX_EVENTS);
		const text = (at: number, len: number) => {
			const field = this.bytes.subarray(at, at + len);
			const end = field.indexOf(0);
			return new TextDecoder().decode(end < 0 ? field : field.subarray(0, end));
		};
		const out: ModuleEvent[] = [];
		for (let i = 0; i < n; i++) {
			const base = this.io + OFFSETS.events + i * OFFSETS.event.size;
			const props: Record<string, number> = {};
			const nprops = this.view.getUint32(base + OFFSETS.event.nprops, true);
			for (let k = 0; k < Math.min(nprops, 4); k++) {
				props[text(base + OFFSETS.event.keys + k * PROP_KEY_LEN, PROP_KEY_LEN)] = this.view.getFloat32(base + OFFSETS.event.values + k * 4, true);
			}
			out.push({
				name: text(base + OFFSETS.event.name, EVENT_NAME_LEN),
				start: this.view.getFloat64(base + OFFSETS.event.start, true),
				len: this.view.getFloat64(base + OFFSETS.event.len, true),
				port: this.view.getUint32(base + OFFSETS.event.port, true),
				props
			});
		}
		return out;
	}
}
