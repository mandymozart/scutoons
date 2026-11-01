// noise.ts (#197, #191): the noise the browser deck plays itself – live-core has no noise kind – for the fixed patch's
// incense and a rig's noise nodes (deck-space rig/engine.rs hands them to the page with their strip gain).

/** a node the page plays itself: pink noise through a lowpass (the fixed patch's incense, a rig's noise nodes) */
export interface PageNoise {
	src: AudioBufferSourceNode;
	filter: BiquadFilterNode;
	gain: GainNode;
	/** its mixer strip's gain (a rig's; 1 for the fixed patch) */
	strip: number;
}

/** Pink-ish noise (three leaky integrators of white noise – cathedral.toml's incense is pink) through a lowpass, at
 *  `gain` × `strip`, into the context's output. */
export function pinkNoise(ctx: AudioContext, cutoff: number, gain: number, strip: number): PageNoise {
	const buf = ctx.createBuffer(1, ctx.sampleRate * 2, ctx.sampleRate);
	const d = buf.getChannelData(0);
	let b0 = 0,
		b1 = 0,
		b2 = 0;
	for (let i = 0; i < d.length; i++) {
		const w = Math.random() * 2 - 1;
		b0 = 0.99765 * b0 + w * 0.099;
		b1 = 0.963 * b1 + w * 0.2965;
		b2 = 0.57 * b2 + w * 1.0527;
		d[i] = (b0 + b1 + b2 + w * 0.1848) * 0.2;
	}
	const src = Object.assign(ctx.createBufferSource(), { buffer: buf, loop: true });
	const filter = Object.assign(ctx.createBiquadFilter(), { type: 'lowpass' as BiquadFilterType });
	filter.frequency.value = cutoff;
	const g = ctx.createGain();
	g.gain.value = gain * strip;
	src.connect(filter).connect(g).connect(ctx.destination);
	src.start();
	return { src, filter, gain: g, strip };
}
