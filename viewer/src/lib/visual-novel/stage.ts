// VN stage: where the characters joined as `portrait` stand (DESIGN.md → "VN layout"; `avatar` ones are the left slot
// of the dialogue box). The stage is on the right, front to back = left to right: a character that goes into the back is
// smaller, higher, dimmer and further to the right. The boxes don't overlap while the front one can stay at least 7 rem
// (4 portraits at 375 px, 5 at 500 px); a more crowded stage keeps that size and stacks: the same order, the steps to the
// right shrink evenly so the lineup still fits the width (the one in front is drawn on top).

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
	/** more portraits than fit side by side: they overlap (stacked) */
	crowded: boolean;
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
	/** below this front size the stage stacks instead of shrinking further (rem) */
	minFrontRem: 7,
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
	const ideal = Math.max(0, Math.min(width * STAGE.maxWidthShare, STAGE.maxRem * rem, height * STAGE.maxHeightShare));
	const minFront = Math.min(ideal, STAGE.minFrontRem * rem);
	const crowded = fit < minFront;
	const front = crowded ? minFront : Math.min(ideal, fit);
	const sizes = scales.map((sc) => Math.floor(front * sc));
	// the steps between the portraits: side by side, or (crowded) shrunk evenly so the front one ends at the left margin
	const steps = sizes.slice(1).reduce((sum, size) => sum + size + gap, 0);
	const squeeze = crowded && steps > 0 ? Math.max(0, width - 2 * margin - sizes[0]) / steps : 1;
	const places: StagePlace[] = [];
	let right = margin;
	for (let k = count - 1; k >= 0; k--) {
		places[k] = { depth: k, size: sizes[k], right: Math.round(right), lift: Math.round(k * STAGE.lift * rem), brightness: Math.max(0.5, 1 - STAGE.dim * k), crowded };
		right += (sizes[k] + gap) * squeeze;
	}
	return places;
}
