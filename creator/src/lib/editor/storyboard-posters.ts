// The variants board's pictures (#125; the port of #62's posters.js – agents/research/VARIANTS-CANVAS.md §3.3, §5
// "posters only, for the bands in view"). A card loads its file once it comes within a screen of the board's view (one
// IntersectionObserver per scroller – the element marked `data-board-scroll`). A video's poster is the <video> itself
// at its first frame (the creator has no thumbnail server: the files live in the member's folder or the work storage).
// The HOVER SCRUB (§3.3: "a video scrubs under the pointer (x = time)"): the pointer's share of the picture's width →
// the time there, the card's own <video> seeks to it – only the hovered card, nothing plays. A video's frame as a still
// (More ▾, §3.7 frame → still): cut in the browser – a <video> + a canvas – into a PNG (cutFrame). The pure parts are
// tested in test/storyboard-posters.test.ts.

/** the pointer's share `f` of a picture's width (0 … 1) → the time there (ms; 0 when the length is unknown) */
export function scrubAt(f: number, ms: number | null | undefined): { share: number; ms: number } {
	const x = Math.min(1, Math.max(0, Number(f) || 0));
	return { share: x, ms: ms && ms > 0 ? x * ms : 0 };
}
/** a time in ms → "1.9 s" (one decimal – the length chip's form) */
export const secondsText = (ms: number) => `${(Math.max(0, ms) / 1000).toFixed(1)} s`;
/** a length in ms → "5 s" / "6.2 s"; '' without one */
export const lengthText = (ms: number | null | undefined) => (ms && ms > 0 ? `${Number((ms / 1000).toFixed(1))} s` : '');

type Waiting = Map<Element, () => void>;
const observers = new WeakMap<Element, { io: IntersectionObserver; waiting: Waiting }>();
/** a Svelte action: `use:near={() => …}` – called once, when the element comes within a screen of its scroller's view */
export function near(el: Element, cb: () => void) {
	const root = el.closest('[data-board-scroll]');
	if (typeof IntersectionObserver === 'undefined' || !root) {
		cb();
		return {};
	}
	let o = observers.get(root);
	if (!o) {
		const waiting: Waiting = new Map();
		const io = new IntersectionObserver(
			(entries) => {
				for (const e of entries) {
					if (!e.isIntersecting) continue;
					const go = waiting.get(e.target);
					waiting.delete(e.target);
					io.unobserve(e.target);
					go?.();
				}
			},
			{ root, rootMargin: '100% 50%' }
		);
		o = { io, waiting };
		observers.set(root, o);
	}
	o.waiting.set(el, cb);
	o.io.observe(el);
	const own = o;
	return {
		destroy: () => {
			own.waiting.delete(el);
			own.io.unobserve(el);
		}
	};
}

/** a video's frame at `seconds` as a PNG blob – in the browser (a <video> + a canvas); its full size */
export async function cutFrame(url: string, seconds: number | ((duration: number) => number)): Promise<Blob> {
	const video = document.createElement('video');
	video.muted = true;
	video.preload = 'auto';
	video.src = url;
	try {
		await new Promise<void>((ok, no) => {
			video.onloadeddata = () => ok();
			video.onerror = () => no(new Error('The video can\'t be read.'));
		});
		const at = typeof seconds === 'function' ? seconds(Number.isFinite(video.duration) ? video.duration : 0) : seconds;
		await new Promise<void>((ok, no) => {
			video.onseeked = () => ok();
			video.onerror = () => no(new Error('The video can\'t be read.'));
			video.currentTime = Math.max(0, at);
		});
		const canvas = document.createElement('canvas');
		canvas.width = video.videoWidth;
		canvas.height = video.videoHeight;
		canvas.getContext('2d')!.drawImage(video, 0, 0);
		return await new Promise<Blob>((ok, no) => canvas.toBlob((b) => (b ? ok(b) : no(new Error('No frame.'))), 'image/png'));
	} finally {
		video.removeAttribute('src');
		video.load();
	}
}
