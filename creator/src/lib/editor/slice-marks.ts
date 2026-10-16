// What happens on a slice / in a group, for the marks in the chapter tree (owner 2026-10-16: "indicators also for other
// placements of items or audio or events … easier to find where things are happening"). Counts, no judgement.

export interface Marks {
	/** text boxes */
	texts: number;
	/** placed items (catalog items on the art) */
	items: number;
	/** slices with audio */
	audio: number;
	/** slices with a video */
	video: number;
	/** map places the story touches (unlockLocation / moveAvatar) */
	places: number;
	/** VN events: the slices', the placed items' tap events and the group's own */
	events: number;
}

const list = (value: unknown): any[] => (Array.isArray(value) ? value : []);
const PLACE_EVENTS = new Set(['unlockLocation', 'moveAvatar']);
const places = (events: any[]) => events.filter((e) => PLACE_EVENTS.has(e?.type)).length;

export const NO_MARKS: Marks = { texts: 0, items: 0, audio: 0, video: 0, places: 0, events: 0 };

export function sliceMarks(slice: any): Marks {
	const items = list(slice?.items);
	const events = [...list(slice?.events), ...items.flatMap((i) => list(i?.events))];
	return {
		texts: list(slice?.dialog).length,
		items: items.length,
		audio: slice?.audio ? 1 : 0,
		video: slice?.video ? 1 : 0,
		places: places(events),
		events: events.length
	};
}

/** a group's slices added up, plus its own VN script */
export function groupMarks(group: any): Marks {
	const sum = { ...NO_MARKS };
	for (const slice of list(group?.slices)) {
		const s = sliceMarks(slice);
		for (const key of Object.keys(sum) as (keyof Marks)[]) sum[key] += s[key];
	}
	const own = list(group?.events);
	sum.events += own.length;
	sum.places += places(own);
	return sum;
}
