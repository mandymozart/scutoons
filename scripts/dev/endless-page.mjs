// The endless band read in the page (FEATURES 10.100, viewer renderer/strip-endless.svelte) – shared by
// endless-check.mjs (headless Chrome over CDP) and endless-engines-check.mjs (Playwright: Chromium, Firefox, WebKit).
// The page sources go in before the document starts; the helpers below them are plain functions on what they read.
import { inflateSync } from 'node:zlib';

/** `window.__endless`: the band – the window, each frame's left in it, the track's translate; points, screenshots */
export const ENDLESS_PAGE = `window.__endless = {
	band: () => document.querySelector('[data-endless]'),
	state() {
		const band = this.band();
		const win = band?.firstElementChild;
		const track = win?.firstElementChild;
		if (!track) return null;
		const w = win.getBoundingClientRect(), b = band.getBoundingClientRect();
		const frames = [...track.children].map((f) => { const r = f.getBoundingClientRect(); return { l: r.left - w.left, w: r.width }; });
		return { x: new DOMMatrix(getComputedStyle(track).transform).m41, T: frames.reduce((s, f) => s + f.w, 0), frames, win: { x: w.left, y: w.top, w: w.width, h: w.height }, band: { x: b.left, y: b.top, w: b.width, h: b.height }, y: scrollY, sw: document.documentElement.scrollWidth, cw: document.documentElement.clientWidth };
	},
	/** every rAF until it rested 40 frames after the pointer let go (a fling's glide) */
	record() {
		const rec = (window.__rec = { states: [], release: -1, done: false });
		addEventListener('pointerup', () => (rec.release = rec.states.length), { capture: true, once: true });
		const loop = () => {
			const s = this.state();
			rec.states.push({ t: performance.now(), x: s.x, T: s.T, frames: s.frames, win: s.win, band: s.band, sw: s.sw, cw: s.cw });
			const n = rec.states.length;
			const still = rec.release >= 0 && n - rec.release > 40 && rec.states.slice(-40).every((r) => r.x === s.x);
			if (still || n > 900) rec.done = true;
			else requestAnimationFrame(loop);
		};
		requestAnimationFrame(loop);
		return true;
	},
	/** a point on the band that nothing covers (the header, a notice) – fx / fy shares of its part on screen first */
	point(fx, fy) {
		const band = this.band(), r = band.getBoundingClientRect();
		const top = Math.max(r.top, 0), bottom = Math.min(r.bottom, innerHeight), x = r.left + r.width * fx;
		for (const t of [fy, 0.5, 0.65, 0.35, 0.8, 0.2, 0.9]) {
			const y = top + (bottom - top) * t;
			if (band.contains(document.elementFromPoint(x, y))) return { x, y };
		}
		return null;
	},
	/** hides everything fixed or sticky outside the band (the header's animated logo, notices) for a screenshot – or shows it again */
	bare(on) {
		const band = this.band();
		if (on) for (const el of document.querySelectorAll('body *')) if (!el.contains(band) && !band.contains(el) && /fixed|sticky/.test(getComputedStyle(el).position)) el.style.setProperty('visibility', 'hidden', 'important'), el.setAttribute('data-endless-check', '');
		if (!on) for (const el of document.querySelectorAll('[data-endless-check]')) el.style.removeProperty('visibility'), el.removeAttribute('data-endless-check');
		return true;
	},
	/** the outermost elements past the page's sides (not fixed: those make no page width), counted by tag.class */
	over() {
		const band = this.band(), cw = document.documentElement.clientWidth, past = (e) => { const r = e.getBoundingClientRect(); return r.width > 0 && (r.left < -0.5 || r.right > cw + 0.5); };
		const names = {};
		for (const e of document.querySelectorAll('body *')) {
			if (band?.contains(e) || !past(e) || getComputedStyle(e).position === 'fixed') continue;
			let up = e.parentElement;
			while (up && up !== document.body && !past(up)) up = up.parentElement;
			if (up && up !== document.body) continue;
			const n = e.tagName.toLowerCase() + '.' + (e.classList[0] ?? '');
			names[n] = (names[n] ?? 0) + 1;
		}
		return Object.entries(names).map(([n, k]) => (k > 1 ? n + ' ×' + k : n));
	}
};`;

// the drag hint, recorded from the document's start: each hint line added (moving or still, its frame, when it went) and
// each sideways bounce (Element.animate calls with a `translateX(…rem) scale(…)` keyframe – fx/cues.ts bounceCue);
// `__nudge.hold(true)` keeps html[data-vn-blocking] set until hold(false)
export const NUDGE_PAGE = `(() => {
	const n = (window.__nudge = { sweeps: [], bounces: [], held: false });
	const animate = Element.prototype.animate;
	Element.prototype.animate = function (keyframes, options) {
		const anim = animate.call(this, keyframes, options);
		if (Array.isArray(keyframes) && keyframes.some((k) => /translateX\\([^)]*rem\\) scale/.test(k.transform ?? ''))) {
			// the zoom: just enough for the band's width (the full-bleed band, not the column)
			const zoom = Math.max(...keyframes.map((k) => parseFloat(/scale\\(([\\d.]+)/.exec(k.transform ?? '')?.[1] ?? '1')));
			const fits = zoom <= 1 + (2 * 0.75 * parseFloat(getComputedStyle(document.documentElement).fontSize)) / (document.querySelector('[data-endless]')?.offsetWidth || innerWidth) + 0.006;
			const b = { t: performance.now(), canceled: false, finished: false, zoom, fits };
			anim.addEventListener('cancel', () => (b.canceled = true));
			anim.addEventListener('finish', () => (b.finished = true));
			n.bounces.push(b);
		}
		return anim;
	};
	const root = () => document.documentElement;
	const block = () => n.held && root().dataset.vnBlocking === undefined && (root().dataset.vnBlocking = '');
	n.hold = (on) => ((n.held = on), on ? block() : delete root().dataset.vnBlocking, true);
	// at the document's start there is no root yet
	const watch = () => (new MutationObserver(block).observe(root(), { attributes: true, attributeFilter: ['data-vn-blocking'] }), new MutationObserver((list) => {
		for (const m of list) {
			for (const el of m.addedNodes) if (el.classList?.contains('sc-hint-sweep')) n.sweeps.push({ el, t: performance.now(), gone: null, frame: [...(el.parentElement?.parentElement?.children ?? [])].indexOf(el.parentElement), inside: ((r, w) => !!w && r.left >= w.left - 1 && r.right <= w.right + 1 && r.width > 0)(el.getBoundingClientRect(), el.closest('.window')?.getBoundingClientRect()), moving: [...el.querySelectorAll('path')].some((p) => p.getAnimations().length > 0) });
			for (const el of m.removedNodes) for (const w of n.sweeps) if (w.el === el && w.gone === null) w.gone = performance.now();
		}
	}).observe(document.body, { childList: true, subtree: true }));
	document.body ? watch() : addEventListener('DOMContentLoaded', watch);
	/** the record without elements / animations; a bounce run = calls within 100 ms (onScreen animates several pieces at once) */
	n.read = () => {
		const runs = n.bounces.filter((b, i) => !i || b.t - n.bounces[i - 1].t > 100).map((b) => ({ t: b.t, canceled: n.bounces.filter((x) => Math.abs(x.t - b.t) <= 100).every((x) => x.canceled), finished: b.finished, zoom: b.zoom, fits: b.fits }));
		return { sweeps: n.sweeps.map(({ el, ...w }) => w), runs, hints: root().classList.contains('sc-show-hints') };
	};
})();`;

/** d mod T into (−T/2, T/2] */
export const norm = (d, T) => d - T * Math.round(d / T);
/** how far each frame moved from a to b (mod T) */
export const moves = (a, b) => a.frames.map((f, i) => norm(b.frames[i].l - f.l, a.T));
export const off = (list, expect, tol) => list.some((d) => Math.abs(d - expect) > tol);
export const fmt = (list) => list.map((d) => d.toFixed(2)).join(' ');
/** the frames on screen: side by side in order (N−1 then 0 at the wrap seam), the window covered – '' or what is wrong */
export function gaps(s) {
	const shown = s.frames.map((f, k) => ({ ...f, k })).filter((f) => f.l < s.win.w && f.l + f.w > 0).sort((a, b) => a.l - b.l);
	if (!shown.length) return 'no frame on screen';
	if (shown[0].l > 0.02 || shown.at(-1).l + shown.at(-1).w < s.win.w - 0.02) return `the window not covered (${fmt(shown.map((f) => f.l))})`;
	for (let i = 1; i < shown.length; i++) {
		const a = shown[i - 1], b = shown[i];
		if (b.k !== (a.k + 1) % s.frames.length || Math.abs(a.l + a.w - b.l) > 0.02) return `frame ${a.k} (${a.l.toFixed(2)} + ${a.w.toFixed(2)}) then ${b.k} (${b.l.toFixed(2)})`;
	}
	return '';
}

/** a PNG screenshot decoded (8 bit RGB / RGBA, not interlaced – Chrome's and Playwright's) */
export function png(buf) {
	let w = 0, h = 0, ch = 0;
	const idat = [];
	for (let p = 8; p < buf.length; ) {
		const len = buf.readUInt32BE(p), kind = buf.toString('latin1', p + 4, p + 8), data = buf.subarray(p + 8, p + 8 + len);
		if (kind === 'IHDR') (w = data.readUInt32BE(0)), (h = data.readUInt32BE(4)), (ch = { 2: 3, 6: 4 }[data[9]]);
		if (kind === 'IDAT') idat.push(data);
		p += 12 + len;
	}
	const raw = inflateSync(Buffer.concat(idat));
	const stride = w * ch;
	const px = Buffer.alloc(h * stride);
	for (let y = 0; y < h; y++) {
		const filter = raw[y * (stride + 1)];
		for (let x = 0; x < stride; x++) {
			const i = y * stride + x;
			const a = x >= ch ? px[i - ch] : 0, b = y ? px[i - stride] : 0, c = x >= ch && y ? px[i - stride - ch] : 0;
			const p = a + b - c, pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
			const pred = [0, a, b, (a + b) >> 1, pa <= pb && pa <= pc ? a : pb <= pc ? b : c][filter];
			px[i] = (raw[y * (stride + 1) + 1 + x] + pred) & 255;
		}
	}
	return { w, h, ch, px, at: (x, y) => { const i = (y * w + x) * ch; return [px[i], px[i + 1], px[i + 2]]; } };
}

/** the share of pixels that differ by more than 32 in a channel, and where (device px) */
export function differ(a, b) {
	if (a.w !== b.w || a.h !== b.h) return { share: 1, box: 'another size' };
	let bad = 0, x0 = a.w, x1 = -1, y0 = a.h, y1 = -1;
	for (let y = 0; y < a.h; y++) for (let x = 0; x < a.w; x++) {
		const p = a.at(x, y), q = b.at(x, y);
		if (Math.max(Math.abs(p[0] - q[0]), Math.abs(p[1] - q[1]), Math.abs(p[2] - q[2])) <= 32) continue;
		bad++;
		(x0 = Math.min(x0, x)), (x1 = Math.max(x1, x)), (y0 = Math.min(y0, y)), (y1 = Math.max(y1, y));
	}
	return { share: bad / (a.w * a.h), box: bad ? `x ${x0}–${x1}, y ${y0}–${y1}` : '' };
}
