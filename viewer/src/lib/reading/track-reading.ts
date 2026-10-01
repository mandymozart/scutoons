import type { Attachment } from 'svelte/attachments';
import { SLICE_PLAYER_EVENT, type SlicePlayerDetail } from '@scutoons/reader';

/** A player video counts as watched from this far on (the credits at the end of a music video) */
export const PLAYER_WATCHED = 0.95;

/** A slice counts as read: at least half of it on screen, or – taller than the screen – its bottom edge is */
export function sliceSeen(entry: IntersectionObserverEntry): boolean {
	if (!entry.isIntersecting) return false;
	const rect = entry.boundingClientRect;
	const height = entry.rootBounds?.height ?? window.innerHeight;
	return entry.intersectionRatio >= 0.5 || (rect.top < 0 && rect.bottom <= height);
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
	/** called when the reader got further: 0..1, exactly 1 only at the chapter's end */
	onRead: (position: number) => void;
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

		/** slice `index` read up to `share` (0..1 of the slice) */
		function read(index: number, share = 1) {
			if (!spot) return;
			const end = spot.last && index >= spot.count - 1 && share >= 1;
			const position = end ? 1 : Math.min(0.99, (spot.offset + index + share) / spot.total);
			if (position <= reported) return;
			reported = position;
			spot.onRead(position);
		}

		function onPlayer(event: Event) {
			const detail = (event as CustomEvent<SlicePlayerDetail>).detail;
			const slice = (event.target as HTMLElement | null)?.closest<HTMLElement>('[data-slice-index]');
			const index = Number(slice?.dataset.sliceIndex);
			if (!detail || !Number.isInteger(index)) return;
			const share = detail.state === 'ended' || detail.progress >= PLAYER_WATCHED ? 1 : detail.progress;
			if (share > 0) read(index, share);
		}
		node.addEventListener(SLICE_PLAYER_EVENT, onPlayer);

		const io = new IntersectionObserver(
			(entries) => {
				for (const entry of entries) {
					if (sliceSeen(entry)) {
						// a player video is read by watching it (onPlayer)
						if ((entry.target as HTMLElement).querySelector('video[data-player]')) continue;
						const index = Number((entry.target as HTMLElement).dataset.sliceIndex);
						if (Number.isInteger(index)) read(index);
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
