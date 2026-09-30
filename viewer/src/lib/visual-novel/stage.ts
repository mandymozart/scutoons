// VN stage: where the characters joined as `portrait` stand (DESIGN.md → "VN layout"; `avatar` ones are the left slot
// of the dialogue box). The stage is on the right, front to back = left to right: a character that goes into the back is
// smaller, higher, dimmer and further to the right. The boxes never overlap, so 1–4 characters stay readable at 375 px.

/** One portrait on the stage (px, relative to the viewport's right / bottom edge). */
export interface StagePlace {
	/** 0 = front (the speaker), 1, 2 … = further back */
	depth: number;
	/** width = height of the portrait box */
	size: number;
	right: number;
	/** raise above the stage floor */
	lift: number;
	brightness: number;
}

export const STAGE = {
	/** each step back: 15 % smaller … */
	shrink: 0.15,
	/** … never below half the front size */
	minScale: 0.5,
	/** … 0.75 rem higher */
	lift: 0.75,
	/** … and dimmer */
	dim: 0.12,
	/** the front portrait at most 55 % of the viewport width, 20 rem, 45 % of its height */
	maxWidthShare: 0.55,
	maxRem: 20,
	maxHeightShare: 0.45,
	/** side margin and the gap between two portraits (rem) */
	margin: 1,
	gap: 0.375
} as const;

/**
 * The places of `count` portraits (index = depth, 0 = front) in a viewport of `width` × `height` px with a root font size
 * of `rem` px: the backmost at the right margin, each one further forward to its left, without overlapping.
 */
export function stageLayout(count: number, width: number, height: number, rem = 16): StagePlace[] {
	if (count <= 0) return [];
	const scales = Array.from({ length: count }, (_, k) => Math.max(STAGE.minScale, 1 - STAGE.shrink * k));
	const margin = STAGE.margin * rem;
	const gap = STAGE.gap * rem;
	const fit = (width - 2 * margin - (count - 1) * gap) / scales.reduce((a, b) => a + b, 0);
	const front = Math.max(0, Math.min(width * STAGE.maxWidthShare, STAGE.maxRem * rem, height * STAGE.maxHeightShare, fit));
	const places: StagePlace[] = [];
	let right = margin;
	for (let k = count - 1; k >= 0; k--) {
		const size = Math.floor(front * scales[k]);
		places[k] = { depth: k, size, right: Math.round(right), lift: Math.round(k * STAGE.lift * rem), brightness: Math.max(0.5, 1 - STAGE.dim * k) };
		right += size + gap;
	}
	return places;
}
