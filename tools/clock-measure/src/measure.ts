// measure.ts (#321, plan §3.7 / P8 "owner LAN measurement ≤ ±5 ms p95"): how far apart two decks click. Both play the
// show clock's click – the native deck at 1 kHz (1.5 kHz on the bar's first beat, the `click` node's defaults), the
// browser deck at 2.5 kHz (3.5 kHz; its fixed scene) – and one microphone records them (a phone's voice memo, a laptop;
// the mic as far from each speaker: 1 m of difference is 2.9 ms). Each deck's band is filtered out of the recording,
// every click's onset found, each onset of deck A paired with deck B's nearest, and the signed offsets (A − B, ms)
// reported: median, mean, p95 of their size, max. The filters' own delay is measured on a synthetic click of the same
// shape and taken out, so both bands are on one time axis.
//
// Pure functions, no file system (the CLI, bin/measure.ts, reads the file).

export interface Audio {
	rate: number;
	/** the channels mixed down to one */
	samples: Float32Array;
}

/** A WAV file (PCM 16 / 24 / 32 bit, float 32, also WAVE_FORMAT_EXTENSIBLE) mixed down to mono. */
export function parseWav(b: Uint8Array): Audio {
	const v = new DataView(b.buffer, b.byteOffset, b.byteLength);
	const tag = (o: number) => String.fromCharCode(b[o], b[o + 1], b[o + 2], b[o + 3]);
	if (tag(0) !== 'RIFF' || tag(8) !== 'WAVE') throw new Error('not a WAV file (RIFF / WAVE)');
	let o = 12;
	let fmt: { format: number; channels: number; rate: number; bits: number } | null = null;
	while (o + 8 <= b.length) {
		const id = tag(o);
		const size = v.getUint32(o + 4, true);
		const body = o + 8;
		if (id === 'fmt ') {
			let format = v.getUint16(body, true);
			// WAVE_FORMAT_EXTENSIBLE: the real format is the sub-format GUID's first two bytes
			if (format === 0xfffe && size >= 26) format = v.getUint16(body + 24, true);
			fmt = { format, channels: v.getUint16(body + 2, true), rate: v.getUint32(body + 4, true), bits: v.getUint16(body + 14, true) };
		} else if (id === 'data') {
			if (!fmt) throw new Error('WAV: data before fmt');
			const { format, channels, rate, bits } = fmt;
			const bytes = bits / 8;
			const frames = Math.floor(Math.min(size, b.length - body) / (bytes * channels));
			const read =
				format === 3 && bits === 32
					? (p: number) => v.getFloat32(p, true)
					: format === 1 && bits === 16
						? (p: number) => v.getInt16(p, true) / 32768
						: format === 1 && bits === 24
							? (p: number) => ((b[p] | (b[p + 1] << 8) | (b[p + 2] << 16)) << 8) / 2147483648
							: format === 1 && bits === 32
								? (p: number) => v.getInt32(p, true) / 2147483648
								: null;
			if (!read) throw new Error(`WAV: format ${format} with ${bits} bits is not read (PCM 16 / 24 / 32, float 32)`);
			const samples = new Float32Array(frames);
			for (let f = 0; f < frames; f++) {
				let s = 0;
				for (let c = 0; c < channels; c++) s += read(body + (f * channels + c) * bytes);
				samples[f] = s / channels;
			}
			return { rate, samples };
		}
		o = body + size + (size & 1);
	}
	throw new Error('WAV: no data chunk');
}

/** A deck's click band (Hz): what its click plays, on the bar's first beat too. */
export interface Band {
	lo: number;
	hi: number;
	/** the click's frequency (the synthetic click the filter's delay is measured on) */
	freq: number;
}

/** the native deck's click (1 kHz, accent 1.5 kHz) and the browser deck's (2.5 kHz, accent 3.5 kHz) */
export const NATIVE: Band = { lo: 800, hi: 1800, freq: 1000 };
export const BROWSER: Band = { lo: 2200, hi: 4200, freq: 2500 };

/** one RBJ biquad section run over `x` (`hp`: high-pass, else low-pass), Q = 1/√2 */
function biquad(x: Float32Array, rate: number, f: number, hp: boolean): Float32Array {
	const w = (2 * Math.PI * f) / rate;
	const alpha = Math.sin(w) / Math.SQRT2;
	const c = Math.cos(w);
	const b0 = hp ? (1 + c) / 2 : (1 - c) / 2;
	const b1 = hp ? -(1 + c) : 1 - c;
	const a0 = 1 + alpha;
	const [nb0, nb1, nb2, na1, na2] = [b0 / a0, b1 / a0, b0 / a0, (-2 * c) / a0, (1 - alpha) / a0];
	const y = new Float32Array(x.length);
	let [x1, x2, y1, y2] = [0, 0, 0, 0];
	for (let i = 0; i < x.length; i++) {
		const v = nb0 * x[i] + nb1 * x1 + nb2 * x2 - na1 * y1 - na2 * y2;
		[x2, x1, y2, y1] = [x1, x[i], y1, v];
		y[i] = v;
	}
	return y;
}

/** `x` through the band: two high-pass and two low-pass sections */
export function bandpass(x: Float32Array, rate: number, b: Band): Float32Array {
	return biquad(biquad(biquad(biquad(x, rate, b.lo, true), rate, b.lo, true), rate, b.hi, false), rate, b.hi, false);
}

/** the engine's click (deck-engine click.rs): a sine with a 30 ms decay */
export function click(rate: number, freq: number, at: number, length: number, gain = 0.5): Float32Array {
	const x = new Float32Array(Math.round(length * rate));
	const start = Math.round(at * rate);
	for (let i = start; i < x.length; i++) {
		const t = (i - start) / rate;
		x[i] = gain * Math.sin(2 * Math.PI * freq * t) * Math.exp(-t / 0.03);
	}
	return x;
}

/**
 * The onsets (s) in a band-passed signal: where its size first passes `share` of the loud clicks' level (the 99.8th
 * percentile) after `hold` s below it (a deck's clicks are a beat apart – ≥ 0.2 s at 300 BPM).
 */
export function onsets(y: Float32Array, rate: number, share = 0.3, hold = 0.12): number[] {
	// the size over the last 1.5 ms: the other deck's click starts with a short broadband edge that leaks through the
	// filters; a click in the band builds up over its cycles
	const n = Math.max(1, Math.round(0.0015 * rate));
	const size = new Float32Array(y.length);
	let sum = 0;
	for (let i = 0; i < y.length; i++) {
		sum += Math.abs(y[i]) - (i >= n ? Math.abs(y[i - n]) : 0);
		size[i] = sum / n;
	}
	const sorted = Float32Array.from(size).sort();
	const loud = sorted[Math.floor(sorted.length * 0.998)] || 0;
	if (loud <= 0) return [];
	const cut = share * loud;
	const gap = Math.round(hold * rate);
	const out: number[] = [];
	let below = gap;
	for (let i = 0; i < size.length; i++) {
		if (size[i] >= cut) {
			if (below >= gap) out.push(i / rate);
			below = 0;
		} else below++;
	}
	return out;
}

/** how late a band's onset finds a click (s): the filters and the threshold on a synthetic click of its frequency */
export function delayOf(rate: number, b: Band): number {
	const at = 0.05;
	const found = onsets(bandpass(click(rate, b.freq, at, 0.3), rate, b), rate)[0];
	return found === undefined ? 0 : found - at;
}

export interface Report {
	/** onsets found per deck, beats paired */
	a: number;
	b: number;
	pairs: number;
	/** A − B per paired beat (ms) */
	offsets: number[];
	median: number;
	mean: number;
	/** of |A − B| */
	p95: number;
	max: number;
	/** p95 within the bound */
	within: boolean;
}

const quantile = (s: number[], q: number) => (s.length ? s[Math.min(s.length - 1, Math.floor(s.length * q))] : NaN);

/** Each onset of A with B's nearest within `window` s: the signed offsets, against `bound` ms. */
export function report(a: number[], b: number[], window = 0.1, bound = 5): Report {
	const offsets: number[] = [];
	let j = 0;
	for (const t of a) {
		while (j + 1 < b.length && Math.abs(b[j + 1] - t) <= Math.abs(b[j] - t)) j++;
		if (b.length && Math.abs(b[j] - t) <= window) offsets.push((t - b[j]) * 1e3);
	}
	const sorted = [...offsets].sort((p, q) => p - q);
	const size = offsets.map(Math.abs).sort((p, q) => p - q);
	const p95 = quantile(size, 0.95);
	return {
		a: a.length,
		b: b.length,
		pairs: offsets.length,
		offsets,
		median: quantile(sorted, 0.5),
		mean: offsets.reduce((s, x) => s + x, 0) / (offsets.length || NaN),
		p95,
		max: size[size.length - 1] ?? NaN,
		within: p95 <= bound
	};
}

/** a recording → the two decks' offsets (deck A − deck B) */
export function measure(wav: Uint8Array, o: { a?: Band; b?: Band; window?: number; bound?: number } = {}): Report {
	const audio = parseWav(wav);
	const [a, b] = [o.a ?? NATIVE, o.b ?? BROWSER].map((band) => {
		const d = delayOf(audio.rate, band);
		return onsets(bandpass(audio.samples, audio.rate, band), audio.rate).map((t) => t - d);
	});
	return report(a, b, o.window, o.bound);
}
