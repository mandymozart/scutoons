// Stills (#842, server docs/API.md "Thumbnails"): the pure parts – the centre cut to the still's shape (the deck's
// `fitted`), what needs making again, when a workspace snapshot is due. Tests: test/stills.test.ts.
import { THUMB_HEIGHT, THUMB_WIDTH, type Thumb } from '@scutoons/api';

export interface Crop {
	sx: number;
	sy: number;
	sw: number;
	sh: number;
}

/** the part of a w × h picture that fills the still's shape, centred (cover: the longer side is cut) */
export function centreCut(w: number, h: number, outW = THUMB_WIDTH, outH = THUMB_HEIGHT): Crop {
	if (!(w > 0 && h > 0)) return { sx: 0, sy: 0, sw: 0, sh: 0 };
	const want = outW / outH;
	if (w / h > want) {
		const sw = h * want;
		return { sx: (w - sw) / 2, sy: 0, sw, sh: h };
	}
	const sh = w / want;
	return { sx: 0, sy: (h - sh) / 2, sw: w, sh };
}

/**
 * Whether this browser should make the still: the server answered (`available`), and it has none, or a stale one, or one
 * taken from another stamp than `stamp` (when the caller knows what the thing is now).
 */
export function wantsStill(available: boolean, thumb: Thumb | undefined, stamp?: string): boolean {
	if (!available) return false;
	if (!thumb || thumb.stale) return true;
	return stamp !== undefined && stamp !== '' && thumb.stamp !== stamp;
}

/** a workspace's snapshot is taken once the workspace held still this long after the last input */
export const SNAPSHOT_SETTLE_MS = 1500;
/** and not more often than this while it stays in front */
export const SNAPSHOT_EVERY_MS = 20_000;

/** when the next snapshot of the front workspace is due: `settle` after the last input, at least `every` after the last one */
export function snapshotDue(lastInput: number, lastTaken: number | null, settle = SNAPSHOT_SETTLE_MS, every = SNAPSHOT_EVERY_MS): number {
	const settled = lastInput + settle;
	return lastTaken === null ? settled : Math.max(settled, lastTaken + every);
}
