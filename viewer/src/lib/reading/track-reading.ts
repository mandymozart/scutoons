import type { Attachment } from 'svelte/attachments';
import { SLICE_PLAYER_EVENT, type SlicePlayerDetail } from '@scutoons/reader';

/** A player video counts as watched from this far on (the credits at the end of a music video) */
export const PLAYER_WATCHED = 0.95;

/** An endless band (toon/renderer/strip-endless.svelte): its frames come in sideways, in any order */
export const ENDLESS_BAND = '[data-endless]';

/**
 * A slice counts as read: at least half of it on screen, or – taller than the screen – its bottom edge is. A band's
 * frame not by its bottom edge (with the band's top above the screen a 3 px sliver of a frame counted – measured), but
 * – wider than two screens – by half the screen's width of it at half its height.
 */
export function sliceSeen(entry: IntersectionObserverEntry): boolean {
	if (!entry.isIntersecting) return false;
	if (entry.intersectionRatio >= 0.5) return true;
	const rect = entry.boundingClientRect;
	const root = entry.rootBounds;
	if (entry.target.closest(ENDLESS_BAND)) {
		const shown = entry.intersectionRect;
		return shown.width >= (root?.width ?? window.innerWidth) / 2 && shown.height >= rect.height / 2;
	}
	return rect.top < 0 && rect.bottom <= (root?.height ?? window.innerHeight);
}

/**
 * An endless band is reached once its bottom is on screen or scrolled above it – its last slice may never have shown
 * (it sits off screen left of the first): the end of the page (next-bar) and a following script without slices wait
 * for this. Not only once passed: a band that ends the page can't scroll above the screen (measured, 3 engines).
 */
export function bandReached(entry: IntersectionObserverEntry): boolean {
	const bottom = entry.boundingClientRect.bottom;
	if (!entry.isIntersecting) return bottom <= (entry.rootBounds?.top ?? 0);
	return bottom <= (entry.rootBounds?.bottom ?? window.innerHeight) + 1;
}

/**
 * How far a band's frames were seen in a row from its first (page index), `first − 1` for none: frames come in
 * sideways in any order – dragging right shows the last one first, which must not mark the ones between as read
 */
export function seenRun(seen: ReadonlySet<number>, first: number): number {
	let run = first - 1;
	while (seen.has(run + 1)) run++;
	return run;
}

/** The thresholds `sliceSeen` needs */
export const SEEN_THRESHOLDS = [0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1];

/** Where the shown group sits in its chapter (the chapter page shows one group at a time) */
export interface ReadingSpot {
	/** slices of the chapter's groups before this one */
	offset: number;
	/** slices of this group */
	count: number;
	/** slices of the whole chapter */
	total: number;
	/** this is the chapter's last group: its last slice is the chapter's end */
	last: boolean;
	/**
	 * called when the reader got further: 0..1, exactly 1 only at the chapter's end; `index` = the slice that got them
	 * there (its `data-slice-index` on the page – the page's spot, BACKLOG 10.69)
	 */
	onRead: (position: number, index: number) => void;
	/**
	 * called once per slice of this group the reader passed (index in the group): a seen slice and every slice before it
	 * (a fast fling can skip a slice's seen moment; reaching slice k means the ones above were scrolled past). The map's
	 * slice unlocks (agents/MAP.md §8.5) listen here.
	 */
	onSlice?: (index: number) => void;
}

/**
 * Reading progress of the shown group (agents/DESIGN.md "Start button"): a slice counts as read when at least half of it
 * is on screen, or – for a slice taller than the screen – when its bottom edge is. Slices carry `data-slice-index`
 * (`toon/slice.svelte`); slices rendered later (VN groups, lazy renderers) are picked up by a MutationObserver.
 * A player video slice (`SliceVideo.player`) is read by watching it, not by seeing it: its share grows with the video's
 * progress (its `sliceplayer` events) and it is read once the video ended or reached `PLAYER_WATCHED`.
 */
export function trackReading(spot: ReadingSpot | null): Attachment<HTMLElement> {
	return (node) => {
		if (!spot || spot.total <= 0 || spot.count <= 0 || typeof IntersectionObserver === 'undefined') return;
		let reported = -1;
		let passed = -1;

		/** slice `index` read up to `share` (0..1 of the slice) */
		function read(index: number, share = 1) {
			if (!spot) return;
			if (share >= 1 && index > passed && spot.onSlice) {
				for (let i = passed + 1; i <= index && i < spot.count; i++) spot.onSlice(i);
				passed = index;
			}
			const end = spot.last && index >= spot.count - 1 && share >= 1;
			const position = end ? 1 : Math.min(0.99, (spot.offset + index + share) / spot.total);
			if (position <= reported) return;
			reported = position;
			spot.onRead(position, index);
		}

		function onPlayer(event: Event) {
			const detail = (event as CustomEvent<SlicePlayerDetail>).detail;
			const slice = (event.target as HTMLElement | null)?.closest<HTMLElement>('[data-slice-index]');
			const index = Number(slice?.dataset.sliceIndex);
			if (!detail || !Number.isInteger(index)) return;
			const share = detail.state === 'ended' || detail.progress >= PLAYER_WATCHED ? 1 : detail.progress;
			// in a band only a watched video counts (its share would jump past frames never seen)
			if (share >= 1 && slice) readWhole(slice, index);
			else if (share > 0 && !slice?.closest(ENDLESS_BAND)) read(index, share);
		}
		node.addEventListener(SLICE_PLAYER_EVENT, onPlayer);

		/** per endless band: the page indices of its frames seen so far */
		const bands = new WeakMap<Element, Set<number>>();
		/** a slice read whole: in a band only the frames seen in a row from its first count (`seenRun`) */
		function readWhole(slice: Element, index: number) {
			const band = slice.closest(ENDLESS_BAND);
			if (!band) return read(index);
			let seen = bands.get(band);
			if (!seen) bands.set(band, (seen = new Set()));
			seen.add(index);
			const indices = [...band.querySelectorAll<HTMLElement>('[data-slice-index]')].map((el) => Number(el.dataset.sliceIndex));
			const first = Math.min(...indices);
			const run = seenRun(seen, first);
			if (run >= first) read(run);
		}

		const io = new IntersectionObserver(
			(entries) => {
				for (const entry of entries) {
					if (sliceSeen(entry)) {
						// a player video is read by watching it (onPlayer)
						if ((entry.target as HTMLElement).querySelector('video[data-player]')) continue;
						const index = Number((entry.target as HTMLElement).dataset.sliceIndex);
						if (Number.isInteger(index)) readWhole(entry.target, index);
					}
				}
			},
			{ threshold: SEEN_THRESHOLDS }
		);
		const watched = new WeakSet<Element>();
		function scan() {
			for (const el of node.querySelectorAll('[data-slice-index]')) {
				if (watched.has(el)) continue;
				watched.add(el);
				io.observe(el);
			}
		}
		scan();
		const mo = new MutationObserver(scan);
		mo.observe(node, { childList: true, subtree: true });
		return () => {
			node.removeEventListener(SLICE_PLAYER_EVENT, onPlayer);
			mo.disconnect();
			io.disconnect();
		};
	};
}
