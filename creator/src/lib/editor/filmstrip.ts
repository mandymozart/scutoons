// The Filmstrip (lap 2 C3, STORYBOARD.md "One view for the order"): an ordered strip of the open item's parts – a
// storyboard's shots, a chapter's slices, an experience's scenes – in groups, each part a frame in its own shape. One
// panel and one picking for all three; what a part is and how a pick acts stay with its page. Pure: test/filmstrip.test.ts.
import { DEFAULT_SLICE } from '@scutoons/content';

/** one part: its frame (`aspect` = width / height, `src` its picture), its name, a line under it, a value at its end */
export interface FilmFrame {
	id: string;
	label: string;
	detail?: string;
	meta?: string;
	aspect: number;
	src?: string | null;
	/** the hover text */
	title?: string;
}

/** a run of parts under a head (a scene, a group of slices) */
export interface FilmGroup {
	key: string;
	/** the head's number (a scene's), in the mono face */
	number?: string;
	label: string;
	frames: FilmFrame[];
}

/** what a pick says beyond the part: Shift spans, Ctrl / ⌘ adds or takes off */
export interface FilmKeys {
	shiftKey?: boolean;
	ctrlKey?: boolean;
	metaKey?: boolean;
}

/** a frame's width for its height: its own shape, kept between a sliver and a wide band so every row stays readable */
export function frameWidth(aspect: number, height: number, slim = 0.3, wide = 2.2): number {
	const a = Number.isFinite(aspect) && aspect > 0 ? aspect : 1;
	return height * Math.min(Math.max(a, slim), wide);
}

/** a group is open when the member opened it, or – untouched – while it holds the current part */
export function groupOpen(group: FilmGroup, current: string | null, toggled: Readonly<Record<string, boolean>>): boolean {
	return toggled[group.key] ?? group.frames.some((f) => f.id === current);
}

/** a slice's place in the chapter, as the Filmstrip names it */
export const sliceId = (group: number, slice: number): string => `${group}/${slice}`;
export const sliceAt = (id: string): { group: number; slice: number } | null => {
	const m = /^(\d+)\/(\d+)$/.exec(id);
	return m ? { group: Number(m[1]), slice: Number(m[2]) } : null;
};

/** a slice as the chapter document holds it: its picture (a path in the toon's folder), its size, the shot it was cut from */
interface SliceLike {
	image?: string;
	width?: number;
	height?: number;
	shot?: string;
}

/**
 * A chapter's slices as the Filmstrip shows them: a group per slice group (its name), a frame per slice in its picture's
 * shape (the platform's slice while the size is unknown) – its number and the shot it was cut from.
 */
export function sliceStrip(groups: readonly { slices?: readonly (SliceLike | null)[] }[], name: (group: number) => string, picture: (path: string) => string): FilmGroup[] {
	return groups.map((g, gi) => ({
		key: `group-${gi}`,
		label: name(gi),
		frames: (g?.slices ?? []).map((s, si) => ({
			id: sliceId(gi, si),
			label: String(si + 1),
			detail: s?.shot,
			aspect: s?.width && s?.height ? s.width / s.height : DEFAULT_SLICE.width / DEFAULT_SLICE.height,
			src: s?.image ? picture(s.image) : null
		}))
	}));
}
