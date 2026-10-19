// The studio's lanes for the shared Timeline (@scutoons/ui timeline.ts – one linear axis in ms; owner 2026-10-19: "a
// shot's width = its reading time", "the take on the chapter's axis"). Pure – no Svelte, no DOM (test/timeline-lanes.test.mjs).
//   shots: the chapter's shots back to back, each as long as its reading time (api.mjs measureShot – 1 s when unknown),
//          the scenes as markers;
//   take:  the Handheld step – the selected shot's active take at the shot's start, its kept part as the trim; not
//          selectable (a press on it seeks at once – the 3D view shows that moment, as the old take bar did).

/** @typedef {{ id: string, seconds?: number, scene?: string|number, review?: string }} ShotIn */

/** the shots lane, the scene markers and the chapter's span (ms) */
export function shotsLane(/** @type {ShotIn[]} */ shots) {
	const items = [];
	const markers = [];
	let at = 0;
	shots.forEach((s, i) => {
		const length = Math.max(1, Math.round((s.seconds > 0 ? s.seconds : 1) * 1000));
		items.push({ id: s.id, start: at, length, label: s.id, state: s.review ?? '' });
		if (i === 0 || shots[i - 1].scene !== s.scene) markers.push({ at, label: String(s.scene ?? '') });
		at += length;
	});
	return { lane: { id: 'shots', label: 'Shots', items, tall: true }, markers, span: at };
}

/** the take lane of the selected shot (null: no take); `take` in the takes file's seconds, `start` = the shot's start (ms) */
export function takeLane(take, start) {
	const last = take?.samples?.length ? take.samples[take.samples.length - 1].t : 0;
	if (!take || !(last > 0)) return null;
	const length = Math.round(last * 1000);
	return {
		id: 'take',
		label: 'Take',
		step: 1000 / 24,
		selectable: false,
		items: [{ id: take.id, start, length, label: take.id, trim: { in: Math.round(take.in * 1000), out: Math.round(take.out * 1000), min: 100 } }]
	};
}

/** the range to show for a selected shot: the shot – or, with a take (Handheld), the take itself, to trim it (ms) */
export function revealOf(item, take) {
	if (!item) return null;
	return take ? [take.items[0].start, take.items[0].start + take.items[0].length] : [item.start, item.start + item.length];
}
