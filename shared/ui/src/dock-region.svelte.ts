import { resizable } from './resizable.svelte.ts';

/**
 * One docked region's size (agents/DOCKING.md): its width (dragged on its `Divider`, arrow keys, Home / double-click =
 * the normal width, remembered per key) and – for a foldable region – its fold to a rail (`Rail`): dragging narrower
 * than 60 % of the normal width snaps it to the rail, dragging out of the rail opens it again (one direction per drag –
 * owner 2026-10-03: folding and unfolding in the same drag glitched). Left regions (the sidebar, the script outline)
 * remember the fold (`foldKey`); the right dock folds for now (`foldable`, owner 2026-10-06) – it opens unfolded.
 * `foldWhen`: the page needs the room (e.g. a region is maximized) – folded for now, not remembered; unfolding by hand wins
 * until it turns false again.
 *
 *   const region = dockRegion({ key: () => 'sc-sidebar-width:creator', foldKey: () => 'sc-sidebar-folded:creator',
 *     side: 'left', anchor: () => el.getBoundingClientRect().left, normal: () => el.offsetWidth, max: () => innerWidth / 3 });
 *   region.restore();   <Divider size={region} edge="right" … />
 */
export interface DockRegionOptions {
	/** localStorage key of the width ('' = not remembered) */
	key: () => string;
	/** localStorage key of the folded state: the region folds and remembers it */
	foldKey?: () => string;
	/** the region folds without remembering it (default: it folds when it has a `foldKey`) */
	foldable?: () => boolean;
	/** where the region is docked: its divider is on the other side (left region → right edge) */
	side: 'left' | 'right';
	/** px: the region's fixed side (a left region's left start, a right region's right end) */
	anchor: () => number;
	/** px: the normal width (also the minimum) */
	normal: () => number;
	/** px */
	max: () => number;
	foldWhen?: () => boolean;
}

export function dockRegion(options: DockRegionOptions) {
	const foldKey = () => options.foldKey?.() ?? '';
	const canFold = () => (options.foldable ? options.foldable() : !!foldKey());
	/** folded by the person (remembered with a `foldKey`) */
	let foldedByUser = $state(false);
	/** unfolded by hand while `foldWhen` holds */
	let unfoldedByHand = $state(false);
	const pressed = () => !!options.foldWhen?.();
	const folded = $derived(canFold() && (foldedByUser || (pressed() && !unfoldedByHand)));
	if (options.foldWhen)
		$effect(() => {
			if (!pressed()) unfoldedByHand = false;
		});

	function setFolded(value: boolean) {
		if (!canFold()) return;
		if (pressed()) {
			// folded for the page's sake: unfolding is a choice for now, not a remembered setting
			unfoldedByHand = !value;
			if (!foldedByUser) return;
		}
		foldedByUser = value;
		const k = foldKey();
		if (!k) return;
		try {
			if (value) localStorage.setItem(k, '1');
			else localStorage.removeItem(k);
		} catch {
			// not remembered
		}
	}

	let startedFolded = false;
	const threshold = () => options.normal() * 0.6;
	const size = resizable({
		storageKey: options.key,
		edge: options.side === 'left' ? 'right' : 'left',
		anchor: options.anchor,
		min: options.normal,
		max: options.max,
		filter: (x) => {
			if (!canFold()) return;
			if (startedFolded) {
				if (folded && x < threshold()) return 'skip'; // still the rail
				if (folded) setFolded(false);
			} else if (x < threshold()) {
				setFolded(true);
				return 'end';
			}
		},
		onstart: () => {
			startedFolded = folded;
		}
	});

	return {
		/** px set by dragging / keys; null = the normal width */
		get width() {
			return size.width;
		},
		set width(next: number | null) {
			size.width = next;
		},
		get folded() {
			return folded;
		},
		/** the region can fold to a rail */
		get foldable() {
			return canFold();
		},
		/** fold / unfold (the fold buttons, the rail, Ctrl + \) */
		fold(value: boolean) {
			setFolded(value);
		},
		/** reads the remembered width and folded state */
		restore() {
			size.restore();
			const k = foldKey();
			if (!k) return;
			try {
				foldedByUser = localStorage.getItem(k) === '1';
			} catch {
				// not remembered
			}
		},
		/** a remembered width wider than `max` (a smaller window) shrinks to it */
		fit() {
			if (size.width !== null) size.width = Math.min(size.width, Math.round(options.max()));
		},
		start: size.start,
		key: size.key,
		reset: size.reset
	};
}
export type DockRegion = ReturnType<typeof dockRegion>;
