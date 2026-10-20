// The variants board's pictures, lazily (agents/research/VARIANTS-CANVAS.md §5 "posters only, for the bands in view"):
// a card loads its picture once it comes within a screen of the board's view (one IntersectionObserver per scroller –
// the element marked `data-board-scroll`); a video's poster is its first frame as a JPEG – GET /api/thumbs with one
// tile (w = 1: the smallest step, clips.mjs thumbPlan; ffmpeg, cached under _out/<ch>/thumbs/ by the file's mtime +
// size) – or, without ffmpeg, the <video> itself at its first frame (VariantCard).

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

/** a picture height in rem → device pixels (the studio's root font size × the screen's density) */
export function devicePx(rem) {
	const root = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	return rem * root * (window.devicePixelRatio || 1);
}
