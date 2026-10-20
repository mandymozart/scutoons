// Compare's ONE clock (agents/research/VARIANTS-CANVAS.md §3.4 – Frame.io: one Play, one scrubber, the same moment in
// each side; the maths: compare.js, node tests). One clock per open Compare (Compare.svelte makes it, stops it on
// close). It owns the sides' <video> elements while they are attached (CompareSide's `use:clock.attach`): each stands
// at the clock's time; one shorter than the clock holds its last frame (paused at its end); while the clock plays they
// play natively and are put back when they drift (DRIFT_MS, at most every half second – a decode hiccup doesn't make
// them seek on every frame). The clock is as long as the longest attached video (its real duration once its metadata
// is in, the record's length before – a webm without a duration keeps the record's; with none known either – a
// recorder's file – it is seeked to its end once so the browser learns it, and until then it plays along while the
// clock runs and holds where it ends); it stops at its end, Play there starts again at 0. A file that can't be played
// leaves the clock (CompareSide says so). Detaching (a side replaced, Compare closed) releases the element's media at
// once – pause, no src, load() – so at most 4 videos ever hold a decoder, and only while Compare is open (§5 "≤ 4 in
// Compare").
import { advance, clockLength, drifted, sideAt } from './compare.js';

export function createClock() {
	const clock = $state({ t: 0, playing: false, length: 0 });
	/** the attached videos: element → its length (ms) */
	const videos = new Map();
	/** when each was last put back (performance.now) */
	const fixed = new WeakMap();
	/** seeked to its end to learn its length (a webm without a duration) – the clock leaves it alone meanwhile */
	const probing = new WeakSet();
	let raf = 0;
	let last = 0;

	const refresh = () => {
		clock.length = clockLength([...videos.values()]);
		if (clock.t > clock.length) clock.t = clock.length;
	};
	/** one video at the clock's time: playing along, or paused at its moment / its last frame */
	function place(el, len, force) {
		if (el.readyState < 1 || probing.has(el)) return;
		if (!(len > 0)) return along(el, force);
		const at = sideAt(clock.t, len);
		const run = clock.playing && !at.held;
		const now = performance.now();
		const ms = el.currentTime * 1000;
		const off = run ? drifted(ms, at.ms) && now - (fixed.get(el) ?? 0) > 500 : Math.abs(ms - at.ms) > 1;
		if (force || off) {
			el.currentTime = at.ms / 1000;
			fixed.set(el, now);
		}
		// one that ended by itself (its file a hair shorter than the clock's count) stays on its last frame – play() would
		// start it again at 0
		if (run && el.paused && !el.ended) el.play().catch(() => {});
		else if (!run && !el.paused) el.pause();
	}
	/** a video of unknown length: at the clock's time when it is put there, playing while the clock runs – where it
	 *  ends it holds (ended) */
	function along(el, force) {
		if (force) el.currentTime = clock.t / 1000;
		if (clock.playing && el.paused && !el.ended) el.play().catch(() => {});
		else if (!clock.playing && !el.paused) el.pause();
	}
	const sync = (force = false) => {
		for (const [el, len] of videos) place(el, len, force);
	};
	function frame(now) {
		const { t, ended } = advance(clock.t, now - last, clock.length);
		last = now;
		clock.t = t;
		if (ended) return pause();
		sync();
		raf = requestAnimationFrame(frame);
	}

	function play() {
		if (!clock.length || clock.playing) return;
		if (clock.t >= clock.length) clock.t = 0;
		clock.playing = true;
		last = performance.now();
		sync(true);
		raf = requestAnimationFrame(frame);
	}
	function pause() {
		cancelAnimationFrame(raf);
		clock.playing = false;
		sync(true);
	}
	return {
		/** { t, playing, length } – ms */
		get t() { return clock.t; },
		get playing() { return clock.playing; },
		get length() { return clock.length; },
		play,
		pause,
		toggle: () => (clock.playing ? pause() : play()),
		/** the scrubber: every side to that moment (playing on from there if it played) */
		seek(ms) {
			clock.t = Math.min(clock.length, Math.max(0, ms || 0));
			if (clock.playing) last = performance.now();
			sync(true);
		},
		/** a Svelte action on a side's <video>: `use:clock.attach={fallbackMs}` – the record's length until the metadata
		 *  says better; destroyed → the element lets go of its media */
		attach(el, fallback) {
			const finite = () => Number.isFinite(el.duration) && el.duration > 0;
			const fromMeta = () => (finite() ? el.duration * 1000 : fallback > 0 ? fallback : 0);
			const loaded = () => {
				// no length anywhere (a webm without a duration, a record without one): to its end once – the browser
				// learns it there (durationchange)
				if (!finite() && !(fallback > 0) && el.duration === Infinity && !probing.has(el)) {
					probing.add(el);
					el.currentTime = 1e101;
				}
				videos.set(el, fromMeta());
				refresh();
				place(el, videos.get(el), true);
			};
			// its real length known (after the metadata, after the probe at its end): the clock follows, the video back to it
			const changed = () => {
				if (!videos.has(el) || !finite()) return;
				probing.delete(el);
				videos.set(el, fromMeta());
				refresh();
				place(el, videos.get(el), true);
			};
			// the probe's seek done – its length learnt or not (then it plays along): back to the clock
			const seeked = () => {
				if (!probing.delete(el)) return;
				if (finite()) videos.set(el, fromMeta());
				refresh();
				place(el, videos.get(el) ?? 0, true);
			};
			// a file that can't be played sets no length (CompareSide shows why)
			const failed = () => {
				probing.delete(el);
				if (videos.delete(el)) refresh();
			};
			videos.set(el, fallback > 0 ? fallback : 0);
			refresh();
			el.addEventListener('loadedmetadata', loaded);
			el.addEventListener('durationchange', changed);
			el.addEventListener('seeked', seeked);
			el.addEventListener('error', failed);
			if (el.readyState >= 1) loaded();
			return {
				destroy() {
					el.removeEventListener('loadedmetadata', loaded);
					el.removeEventListener('durationchange', changed);
					el.removeEventListener('seeked', seeked);
					el.removeEventListener('error', failed);
					videos.delete(el);
					el.pause();
					el.removeAttribute('src');
					el.load();
					refresh();
					if (!videos.size) pause();
				}
			};
		},
		/** Compare closes: the loop ends (the elements go with their sides) */
		stop() {
			cancelAnimationFrame(raf);
			clock.playing = false;
		}
	};
}
