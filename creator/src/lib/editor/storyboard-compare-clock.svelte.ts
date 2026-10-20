// Compare's ONE clock (#125; the port of #62's compare-clock.svelte.js – agents/research/VARIANTS-CANVAS.md §3.4,
// Frame.io: one Play, one scrubber, the same moment in each side; the maths: storyboard-compare.ts). One clock per open
// Compare (compare-panel makes it, stops it on close). It owns the sides' <video> elements while they are attached
// (`use:clock.attach`): each stands at the clock's time; one shorter than the clock holds its last frame; while the clock
// plays they play natively and are put back when they drift (DRIFT_MS, at most every half second). The clock is as long
// as the longest attached video (its real duration once its metadata is in, the record's length before; a webm without
// a duration is seeked to its end once so the browser learns it). A file that can't be played leaves the clock.
// Detaching (a side replaced, Compare closed) releases the element's media at once – at most 4 videos hold a decoder.
import { advance, clockLength, drifted, sideAt } from './storyboard-compare.ts';

export interface CompareClock {
	readonly t: number;
	readonly playing: boolean;
	readonly length: number;
	play(): void;
	pause(): void;
	toggle(): void;
	seek(ms: number): void;
	attach(el: HTMLVideoElement, fallback: number | null): { destroy(): void };
	stop(): void;
}

export function createClock(): CompareClock {
	const clock = $state({ t: 0, playing: false, length: 0 });
	const videos = new Map<HTMLVideoElement, number>();
	const fixed = new WeakMap<HTMLVideoElement, number>();
	const probing = new WeakSet<HTMLVideoElement>();
	let raf = 0;
	let last = 0;

	const refresh = () => {
		clock.length = clockLength([...videos.values()]);
		if (clock.t > clock.length) clock.t = clock.length;
	};
	function place(el: HTMLVideoElement, len: number, force: boolean) {
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
		if (run && el.paused && !el.ended) el.play().catch(() => {});
		else if (!run && !el.paused) el.pause();
	}
	function along(el: HTMLVideoElement, force: boolean) {
		if (force) el.currentTime = clock.t / 1000;
		if (clock.playing && el.paused && !el.ended) el.play().catch(() => {});
		else if (!clock.playing && !el.paused) el.pause();
	}
	const sync = (force = false) => {
		for (const [el, len] of videos) place(el, len, force);
	};
	function frame(now: number) {
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
		get t() {
			return clock.t;
		},
		get playing() {
			return clock.playing;
		},
		get length() {
			return clock.length;
		},
		play,
		pause,
		toggle: () => (clock.playing ? pause() : play()),
		seek(ms: number) {
			clock.t = Math.min(clock.length, Math.max(0, ms || 0));
			if (clock.playing) last = performance.now();
			sync(true);
		},
		attach(el: HTMLVideoElement, fallback: number | null) {
			const fb = fallback && fallback > 0 ? fallback : 0;
			const finite = () => Number.isFinite(el.duration) && el.duration > 0;
			const fromMeta = () => (finite() ? el.duration * 1000 : fb);
			const loaded = () => {
				if (!finite() && !fb && el.duration === Infinity && !probing.has(el)) {
					probing.add(el);
					el.currentTime = 1e101;
				}
				videos.set(el, fromMeta());
				refresh();
				place(el, videos.get(el)!, true);
			};
			const changed = () => {
				if (!videos.has(el) || !finite()) return;
				probing.delete(el);
				videos.set(el, fromMeta());
				refresh();
				place(el, videos.get(el)!, true);
			};
			const seeked = () => {
				if (!probing.delete(el)) return;
				if (finite()) videos.set(el, fromMeta());
				refresh();
				place(el, videos.get(el) ?? 0, true);
			};
			const failed = () => {
				probing.delete(el);
				if (videos.delete(el)) refresh();
			};
			videos.set(el, fb);
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
		stop() {
			cancelAnimationFrame(raf);
			clock.playing = false;
		}
	};
}
