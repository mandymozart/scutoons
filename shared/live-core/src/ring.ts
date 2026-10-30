// The rings between the threads (#184): one writer, one reader, no locks. Over a SharedArrayBuffer they connect the
// page, the worker and the AudioWorklet without a postMessage; over a plain ArrayBuffer the same code is a local
// queue (the fallback when the page is not cross-origin isolated – the chunks then arrive by postMessage).
//
// The state is an Int32Array [write, read]: counters that only grow (wrapping at 2^32, compared as int32 differences),
// the index into the data is `counter & (capacity − 1)` (capacity is a power of two). The writer publishes its counter
// after the data, the reader after it has copied – each side stores only its own counter (Atomics.store / load, so the
// other thread sees the data before the counter). The reader may `Atomics.notify` the read counter, so a writer can
// `Atomics.waitAsync` for space.

export const WRITE = 0;
export const READ = 1;

type Data = Float32Array | Uint8Array;

/** The two buffers of a ring, as they are posted to the other thread. */
export interface RingBuffers {
	state: SharedArrayBuffer | ArrayBuffer;
	data: SharedArrayBuffer | ArrayBuffer;
}

export const sharedAvailable = (): boolean =>
	typeof SharedArrayBuffer === 'function' && (globalThis as { crossOriginIsolated?: boolean }).crossOriginIsolated !== false;

const pow2 = (n: number): number => {
	if (!Number.isInteger(n) || n < 2 || n > 1 << 30 || (n & (n - 1)) !== 0) throw new RangeError(`ring: capacity ${n} is not a power of two (2 … 2^30)`);
	return n;
};

export function ringBuffers(capacity: number, bytesPerItem: 1 | 4, shared: boolean): RingBuffers {
	const Buf = shared ? SharedArrayBuffer : ArrayBuffer;
	return { state: new Buf(8), data: new Buf(pow2(capacity) * bytesPerItem) };
}

export class Ring<T extends Data> {
	readonly state: Int32Array;
	readonly data: T;
	readonly capacity: number;
	private readonly mask: number;

	constructor(state: Int32Array, data: T) {
		this.state = state;
		this.data = data;
		this.capacity = pow2(data.length);
		this.mask = this.capacity - 1;
	}

	static floats(b: RingBuffers): Ring<Float32Array> {
		return new Ring(new Int32Array(b.state), new Float32Array(b.data));
	}

	static bytes(b: RingBuffers): Ring<Uint8Array> {
		return new Ring(new Int32Array(b.state), new Uint8Array(b.data));
	}

	/** items the reader can take */
	available(): number {
		return (Atomics.load(this.state, WRITE) - Atomics.load(this.state, READ)) | 0;
	}

	/** items the writer can put */
	space(): number {
		return this.capacity - this.available();
	}

	/** Writer: as much of `src` as fits; returns how many items. */
	write(src: T): number {
		const w = Atomics.load(this.state, WRITE);
		const free = this.capacity - ((w - Atomics.load(this.state, READ)) | 0);
		const n = Math.min(free, src.length);
		if (n <= 0) return 0;
		const at = w & this.mask;
		const first = Math.min(n, this.capacity - at);
		this.data.set(src.subarray(0, first) as never, at);
		if (n > first) this.data.set(src.subarray(first, n) as never, 0);
		Atomics.store(this.state, WRITE, (w + n) | 0);
		return n;
	}

	/** Writer: all of `src` or nothing (a record must never arrive in pieces); false = full. */
	writeAll(src: T): boolean {
		if (src.length > this.space()) return false;
		this.write(src);
		return true;
	}

	/** Reader: up to `dst.length` items into `dst`; returns how many. `notify` wakes a writer waiting for space. */
	read(dst: T, notify = false): number {
		const r = Atomics.load(this.state, READ);
		const n = Math.min(dst.length, (Atomics.load(this.state, WRITE) - r) | 0);
		if (n <= 0) return 0;
		const at = r & this.mask;
		const first = Math.min(n, this.capacity - at);
		dst.set(this.data.subarray(at, at + first) as never, 0);
		if (n > first) dst.set(this.data.subarray(0, n - first) as never, first);
		Atomics.store(this.state, READ, (r + n) | 0);
		if (notify) Atomics.notify(this.state, READ); // 0 on a plain buffer, no throw
		return n;
	}
}

/**
 * The newest value of a fixed size (the analysis frame), one writer, any number of readers: a sequence lock. The
 * writer makes the counter odd, copies, makes it even; a reader copies and keeps the copy only when the counter was
 * even and unchanged around it (else it was torn – the next frame comes in 16 ms).
 */
export class Latest {
	readonly seq: Int32Array;
	readonly data: Float32Array;

	constructor(b: RingBuffers) {
		this.seq = new Int32Array(b.state, 0, 1);
		this.data = new Float32Array(b.data);
	}

	static buffers(size: number, shared: boolean): RingBuffers {
		const Buf = shared ? SharedArrayBuffer : ArrayBuffer;
		return { state: new Buf(8), data: new Buf(size * 4) };
	}

	publish(frame: ArrayLike<number>): void {
		const s = Atomics.load(this.seq, 0);
		Atomics.store(this.seq, 0, (s + 1) | 0);
		this.data.set(frame as Float32Array);
		Atomics.store(this.seq, 0, (s + 2) | 0);
	}

	/** The newest frame into `dst` when it is newer than `seen`: its sequence number, or `seen` when nothing new / torn. */
	take(dst: Float32Array, seen: number): number {
		const a = Atomics.load(this.seq, 0);
		if (a === seen || a & 1) return seen;
		dst.set(this.data);
		return Atomics.load(this.seq, 0) === a ? a : seen;
	}
}
