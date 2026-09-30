import type { Attachment } from 'svelte/attachments';

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
 */
export function trackReading(spot: ReadingSpot | null): Attachment<HTMLElement> {
	return (node) => {
		if (!spot || spot.total <= 0 || spot.count <= 0 || typeof IntersectionObserver === 'undefined') return;
		let furthest = -1;

		function read(index: number) {
			if (!spot || index <= furthest) return;
			furthest = index;
			const end = spot.last && index >= spot.count - 1;
			spot.onRead(end ? 1 : Math.min(0.99, (spot.offset + index + 1) / spot.total));
		}

		const io = new IntersectionObserver(
			(entries) => {
				for (const entry of entries) {
					if (!entry.isIntersecting) continue;
					const rect = entry.boundingClientRect;
					const height = entry.rootBounds?.height ?? window.innerHeight;
					if (entry.intersectionRatio >= 0.5 || (rect.top < 0 && rect.bottom <= height)) {
						const index = Number((entry.target as HTMLElement).dataset.sliceIndex);
						if (Number.isInteger(index)) read(index);
					}
				}
			},
			{ threshold: [0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1] }
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
			mo.disconnect();
			io.disconnect();
		};
	};
}
