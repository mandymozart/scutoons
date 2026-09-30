// VN stage (DESIGN.md → "VN layout"): every character in the scene stands on the dialogue box. The view of the joinScene
// ref is the side: `avatar` = the left group (framed avatars), `portrait` = the right group. Each group is anchored at its
// screen edge; front to back = towards the centre → towards the edge. Owner rules:
//   - avatars (left): 4 rem, framed; the front one (the speaker) 0.25 rem inside the box's left border, each one behind
//     just 1 rem further in (a tight stack behind the box);
//   - portraits (right): 9 rem, a little smaller per step back (9 %); the one behind overlaps the one in front by 1/3
//     of that one's (scaled) width; when the group doesn't fit its side, the overlap grows evenly up to 2/3; if 2/3
//     doesn't fit either, the side is scaled down evenly until everyone fits – nobody is ever hidden.
// Depth also shows as stacking order (the speaker in front of the box, everyone else behind it, 1 rem lower) and dimming.

export type StageSide = 'left' | 'right';

/** One character on the stage (px). */
export interface StagePlace {
	side: StageSide;
	/** 0 = front (the speaker of that side, else the first to join), 1, 2 … = further back */
	depth: number;
	/** width = height of the image box (bottom-anchored) */
	size: number;
	/** distance of the box's outer side from the side's screen edge (right edge for `right`, left edge for `left`) */
	offset: number;
}

export interface StageGroup {
	/** front to back (index = depth) */
	places: StagePlace[];
	/** the overlap in use, as a share of the width of the one in front */
	overlap: number;
	/** < 1 when even 2/3 overlap didn't fit and the side was scaled down */
	scale: number;
}

/** per view: size (rem), shrink per step back, default overlap; avatars: a fixed step (rem) instead */
export const VIEWS = {
	avatar: { rem: 4, shrink: 0, overlap: 0, stepRem: 1 },
	portrait: { rem: 9, shrink: 0.09, overlap: 1 / 3, stepRem: 0 }
} as const;

export const STAGE = {
	/** the most overlap when crowded (then "+N") */
	maxOverlap: 2 / 3,
	/** never smaller than half the front size */
	minScale: 0.5,
	/** not speaking: dimmed */
	dim: 0.85,
	/** side margins (rem): the avatars 0.25 rem inside the box's left border (the box is 1 rem from the edge), the
	 * portraits at the box's right border; the gap between the two groups */
	marginLeft: 1.25,
	marginRight: 1,
	centerGap: 0.5,
	/** the ones not speaking stand this much lower, further behind the box (rem) */
	behind: 1,
	/** how far everyone stands into the box (px): grounded, the box covers the cut edge of the art */
	sink: 4
} as const;

type View = (typeof VIEWS)[keyof typeof VIEWS];
const sizesOf = (n: number, view: View, rem: number) =>
	Array.from({ length: n }, (_, k) => view.rem * rem * Math.max(STAGE.minScale, 1 - view.shrink * k));
/** a group's width: every box but the backmost counts with its visible part (1 - overlap), the backmost whole */
const groupWidth = (sizes: number[], overlap: number) =>
	sizes.length ? sizes.slice(0, -1).reduce((w, s) => w + s * (1 - overlap), 0) + sizes[sizes.length - 1] : 0;

/** the avatar stack: the front one at the margin, each one behind a fixed step further in */
function stackGroup(side: StageSide, n: number, view: View, rem: number): StageGroup {
	const margin = (side === 'left' ? STAGE.marginLeft : STAGE.marginRight) * rem;
	const size = view.rem * rem;
	const places = Array.from({ length: n }, (_, k) => ({ side, depth: k, size, offset: +(margin + k * view.stepRem * rem).toFixed(2) }));
	return { places, overlap: n > 1 ? 1 - (view.stepRem * rem) / size : 0, scale: 1 };
}

function placeGroup(side: StageSide, n: number, view: View, region: number, rem: number): StageGroup {
	if (view.stepRem) return stackGroup(side, n, view, rem);
	const margin = (side === 'left' ? STAGE.marginLeft : STAGE.marginRight) * rem;
	let sizes = sizesOf(n, view, rem);
	let overlap: number = view.overlap;
	let scale = 1;
	if (groupWidth(sizes, overlap) > region) {
		if (groupWidth(sizes, STAGE.maxOverlap) <= region) {
			// crowded: the overlap that just fits, the same for the whole group
			const steps = sizes.slice(0, -1).reduce((a, s) => a + s, 0);
			overlap = 1 - (region - sizes[sizes.length - 1]) / steps;
		} else {
			// even 2/3 is too wide: the whole side smaller, evenly, until it fits
			overlap = STAGE.maxOverlap;
			scale = region / groupWidth(sizes, overlap);
			sizes = sizes.map((s) => s * scale);
		}
	}
	const places: StagePlace[] = [];
	// from the back end: the backmost shown one at the margin, each one in front further in
	let offset = margin;
	for (let k = sizes.length - 1; k >= 0; k--) {
		places[k] = { side, depth: k, size: +sizes[k].toFixed(2), offset: +offset.toFixed(2) };
		if (k > 0) offset += sizes[k - 1] * (1 - overlap) - (sizes[k - 1] - sizes[k]);
	}
	return { places, overlap, scale };
}

/** The groups of `left` avatar-view and `right` portrait-view characters in a viewport `width` px wide. */
export function stageLayout(left: number, right: number, width: number, rem = 16): Record<StageSide, StageGroup> {
	const avail = width - (STAGE.marginLeft + STAGE.marginRight) * rem - (left && right ? STAGE.centerGap * rem : 0);
	// each side gets what it needs at its default overlap; when both don't fit, the smaller need is kept (up to half),
	// the other side gets the rest
	const needL = left ? (VIEWS.avatar.rem + (left - 1) * VIEWS.avatar.stepRem) * rem : 0;
	const needR = groupWidth(sizesOf(right, VIEWS.portrait, rem), VIEWS.portrait.overlap);
	let regionL = avail;
	let regionR = avail;
	if (left && right) {
		if (needL + needR <= avail) [regionL, regionR] = [needL, needR];
		else if (needL <= avail / 2) [regionL, regionR] = [needL, avail - needL];
		else if (needR <= avail / 2) [regionL, regionR] = [avail - needR, needR];
		else [regionL, regionR] = [avail / 2, avail / 2];
	}
	return { left: placeGroup('left', left, VIEWS.avatar, regionL, rem), right: placeGroup('right', right, VIEWS.portrait, regionR, rem) };
}

// ── Full view (owner 2026-09-30): the centre group ─────────────────────────────────────────────────────────────────
// Full characters (breaking the 4th wall) stand in the centre of the screen, feet at the dialogue box's BOTTOM edge –
// the box (always on top) covers the lower body, the rest reaches well above it. The front one (the speaker, else the
// most recent) stands exactly centred at full size; the ones behind step back alternating left and right by
// `FULL.stepRem` per step (2nd 1.5 rem left, 3rd 1.5 rem right, 4th 3 rem left …), each step 9 % smaller. The group is
// behind the avatars and portraits (and the box) unless its speaker speaks: then it is in front of the stage, still
// under the box.

export const FULL = {
	/** the front one's height: this share of the viewport height … */
	heightVh: 0.8,
	/** … at most as wide as the viewport minus this margin per side (rem); full images are 1 : 2 (600 × 1200) */
	marginRem: 1,
	aspect: 0.5,
	/** smaller per step back */
	shrink: 0.09,
	/** sideways per step back (rem), alternating left / right */
	stepRem: 1.5,
	/** z: behind the portrait stage (90 − depth) and the avatars; the speaking one in front of the stage, under the box (100) */
	zBehind: 70,
	zSpeaker: 95
} as const;

export interface FullPlace {
	/** 0 = front */
	depth: number;
	/** the image box (px) */
	width: number;
	height: number;
	/** the box's centre, px from the viewport's left edge */
	center: number;
	/** px from the viewport's left edge to the box's left side */
	left: number;
}

/** Sideways step of depth `k` in units of `FULL.stepRem`: 0, −1, +1, −2, +2 … (negative = left). */
export const fullStep = (k: number) => (k === 0 ? 0 : (k % 2 ? -1 : 1) * Math.ceil(k / 2));

/** The places of `n` full characters (index = depth, front to back) in a `width` × `height` px viewport. */
export function fullLayout(n: number, width: number, height: number, rem = 16): FullPlace[] {
	const front = Math.min(height * FULL.heightVh, (width - 2 * FULL.marginRem * rem) / FULL.aspect);
	return Array.from({ length: n }, (_, k) => {
		const h = front * Math.max(STAGE.minScale, 1 - FULL.shrink * k);
		const w = h * FULL.aspect;
		const center = width / 2 + fullStep(k) * FULL.stepRem * rem;
		return { depth: k, width: +w.toFixed(2), height: +h.toFixed(2), center: +center.toFixed(2), left: +(center - w / 2).toFixed(2) };
	});
}
