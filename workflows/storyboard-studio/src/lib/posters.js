// The variants board's pictures, lazily (agents/research/VARIANTS-CANVAS.md §5 "posters only, for the bands in view"):
// a card loads its picture once it comes within a screen of the board's view (one IntersectionObserver per scroller –
// the element marked `data-board-scroll`); a video's poster is its first frame as a JPEG – GET /api/thumbs with one
// tile (w = 1: the smallest step, clips.mjs thumbPlan; ffmpeg, cached under _out/<ch>/thumbs/ by the file's mtime +
// size) – or, without ffmpeg, the <video> itself at its first frame (VariantCard). The HOVER SCRUB (§3.3: "a video scrubs
// under the pointer (x = time)"): one sprite of SCRUB_TILES frames from the same GET /api/thumbs as the timeline's
// filmstrip (ClipStrip), asked on a card's first hover only – a tile under the pointer is a background position, so the
// scrub never waits for a decoder or a seek, and nothing plays (one picture per hovered card, cached on disk).

const posters = new Map();
/** a video's first frame (url of a JPEG) `px` device pixels high – null when the server can't make one */
export function posterOf(url, px) {
	const h = Math.max(32, Math.round(px));
	const key = `${url}|${h}`;
	if (!posters.has(key)) {
		posters.set(key, fetch(`/api/thumbs?url=${encodeURIComponent(url)}&w=1&h=${h}`)
			.then((r) => (r.ok ? r.json() : null))
			.then((j) => (j?.url && j.n === 1 ? j.url : null))
			.catch(() => null));
	}
	return posters.get(key);
}

const observers = new WeakMap();
const waiting = new WeakMap();
/** a Svelte action: `use:near={() => …}` – called once, when the element comes within a screen of its scroller's view */
export function near(el, cb) {
	const root = el.closest('[data-board-scroll]');
	if (typeof IntersectionObserver === 'undefined' || !root) { cb(); return {}; }
	let io = observers.get(root);
	if (!io) {
		io = new IntersectionObserver((entries) => {
			for (const e of entries) {
				if (!e.isIntersecting) continue;
				const go = waiting.get(e.target);
				waiting.delete(e.target);
				io.unobserve(e.target);
				go?.();
			}
		}, { root, rootMargin: '100% 50%' });
		observers.set(root, io);
	}
	waiting.set(el, cb);
	io.observe(el);
	return { destroy: () => { waiting.delete(el); io.unobserve(el); } };
}

/** a poster's height in steps of 1.5× (64, 96, 144 … px): the first at least `px` high – a zoom asks again only when it
 *  passes a step (the server caches each one) */
export function pxStep(px) {
	let h = 64;
	while (h < px && h < 2048) h = Math.round(h * 1.5);
	return h;
}

/** a picture height in rem → device pixels (the studio's root font size × the screen's density) */
export function devicePx(rem) {
	const root = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	return rem * root * (window.devicePixelRatio || 1);
}

/** the scrub sprite's frames – a 5 s clip: one every ~0.16 s */
export const SCRUB_TILES = 32;
/** the pointer's share `f` of a picture's width (0 … 1) → the sprite tile under it (tile j = the frame at j · length / n,
 *  clips.mjs makeSprite) and the time there (ms; 0 when the length is unknown) */
export function scrubAt(f, n, ms) {
	const x = Math.min(1, Math.max(0, Number(f) || 0));
	return { tile: Math.max(0, Math.min(n - 1, Math.floor(x * n))), ms: ms > 0 ? x * ms : 0 };
}
/** a time in ms → "1.9 s" (one decimal – the length chip's form) */
export const secondsText = (ms) => `${(Math.max(0, ms) / 1000).toFixed(1)} s`;

const sprites = new Map();
/** a video's scrub sprite `px` device pixels high: ~SCRUB_TILES tiles at the picture's aspect `wide` (w / h – the server
 *  picks n from it and the video's own size) → { url, n, tw, th }, decoded – null without ffmpeg / for a broken file */
export function spriteOf(url, px, wide, ms) {
	const h = Math.max(32, Math.round(px));
	const key = `${url}|${h}`;
	if (!sprites.has(key)) {
		const q = new URLSearchParams({ url, w: String(Math.ceil(h * Math.max(0.1, wide) * SCRUB_TILES)), h: String(h) });
		if (ms > 0) q.set('ms', String(Math.round(ms)));
		sprites.set(key, fetch(`/api/thumbs?${q}`)
			.then((r) => (r.ok ? r.json() : null))
			.then(async (j) => {
				if (!j?.url || !(j.n > 1)) return null;
				const img = new Image();
				img.src = j.url;
				await img.decode();
				return j;
			})
			.catch(() => null));
	}
	return sprites.get(key);
}
