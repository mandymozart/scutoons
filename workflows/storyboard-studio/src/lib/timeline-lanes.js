// The studio's clips lane for the shared Timeline (@scutoons/ui timeline.ts – one linear axis in ms). Owner 2026-10-19:
// "every scene is first of all spatial in the webtoon … depending on how long a camera is recorded and or trimmed to,
// thats the length of that clip for that shot", "the other timeline underneath the clip only shows those shots that are
// selected". Pure – no Svelte, no DOM (test/timeline-lanes.test.mjs).
// ONE lane: the PICKED shots back to back in toon order, each ONE clip – the clip of the view mode, falling back
// generated → rendered → take → still (research/TIMELINE.md §1; the clips from GET /api/clips, clips.mjs):
//   generated: the shot's generated video – the approved one, else the newest (clips.mjs lists the newest first);
//   rendered:  the newest handheld render of the active take, its beauty pass;
//   take:      the active take's kept part (out − in); in Take mode the whole take with its trim (the in / out handles);
//   still:     STILL_MS (owner: "those without a recorded or rendered video will need a default length").
// Shot mode = every shot a still. `clip.from` = where the clip's media is at the item's start (ms): a take's in outside
// Take mode, else 0 – the media time under the playhead is `from + off`.

/** a shot without a recorded / rendered / generated clip (owner: "perhapse 5 seconds or 15 sec" – 5) */
export const STILL_MS = 5000;
/** a trim's shortest kept part */
export const TRIM_MIN = 100;
/** the view modes, like Blender's viewport shading (owner: "in our case, it is shot/take/rendered/generated") */
export const MODES = ['shot', 'take', 'rendered', 'generated'];
const CHAIN = { shot: ['still'], take: ['take', 'still'], rendered: ['rendered', 'take', 'still'], generated: ['generated', 'rendered', 'take', 'still'] };

/** a clip the main view plays as a video (the rest is the 3D view) */
export const isVideo = (source) => source === 'rendered' || source === 'generated';

/** @typedef {{ id: string, in: number, out: number, length: number, kept: number }} TakeIn – ms, as clips.mjs activeTake */
/** @typedef {{ take?: TakeIn|null, rendered?: { file: string, url: string, pass: string, take: string, ms: number|null }[], generated?: { file: string, url: string, ms: number|null, video: boolean, approved: boolean }[] }} ClipsIn */

/** a video's length: the API's, else the one read from its metadata (`known`), else unknown (null) */
const msOf = (v, known) => (v.ms > 0 ? v.ms : known[v.url] > 0 ? known[v.url] : null);
function video(source, v, known, take) {
	const ms = msOf(v, known);
	return { source, url: v.url, file: v.file, length: ms ?? STILL_MS, from: 0, take, probe: ms == null };
}

/** one shot's clip in a view mode: { source, length, from, url?, file?, take, probe } – `probe`: a video of unknown
 *  length (it counts STILL_MS until its metadata is read) */
export function clipOf(/** @type {ClipsIn|undefined} */ entry, mode, known = {}) {
	const take = entry?.take?.length > 0 ? entry.take : null;
	for (const source of CHAIN[mode] ?? CHAIN.shot) {
		if (source === 'generated') {
			const videos = (entry?.generated ?? []).filter((g) => g.video);
			const g = videos.find((x) => x.approved) ?? videos[0];
			if (g) return video(source, g, known, take);
		} else if (source === 'rendered') {
			const r = take && (entry?.rendered ?? []).find((x) => x.take === take.id && x.pass === 'beauty');
			if (r) return video(source, r, known, take);
		} else if (source === 'take') {
			if (!take) continue;
			const whole = mode === 'take';
			return { source, length: whole ? take.length : Math.max(1, take.kept), from: whole ? 0 : take.in, take, probe: false };
		}
	}
	return { source: 'still', length: STILL_MS, from: 0, take, probe: false };
}

/** the clips lane: `ids` = the picked shots in toon order, `clips` = GET /api/clips, `known` = lengths read from the
 *  videos (url → ms), `live` = { [shot]: take | null } – the current shot's takes file as it is edited (ms) */
export function clipsLane(/** @type {string[]} */ ids, mode, clips = {}, known = {}, live = {}) {
	const items = [];
	const probe = [];
	let at = 0;
	for (const id of ids) {
		const entry = id in live ? { ...clips[id], take: live[id] } : clips[id];
		const clip = clipOf(entry, mode, known);
		if (clip.probe) probe.push(clip.url);
		// the label is the item's accessible name: the shot and which clip it is (the tag shows both)
		const item = { id, start: at, length: clip.length, label: `${id} · ${clip.source}`, state: clip.source, clip };
		if (mode === 'take' && clip.source === 'take') item.trim = { in: clip.take.in, out: clip.take.out, min: TRIM_MIN };
		items.push(item);
		at += clip.length;
	}
	return { lane: { id: 'clips', label: 'Clips', items, tall: true, step: 1000 / 24 }, span: at, probe };
}

/** the clip under `t`: { item, off } – [start, end); from the end of the last one on, the last one at its end */
export function clipAt(items, t) {
	const item = items.find((i) => t >= i.start && t < i.start + i.length) ?? (items.length && t >= items.at(-1).start ? items.at(-1) : null);
	return item ? { item, off: Math.min(t - item.start, item.length) } : null;
}

/** `t` out of a cut part (Play plays the kept parts): before a trim's in → the in, at / past its out → its clip's end
 *  (the next clip's start – and that one's in, when it is cut too) */
export function kept(items, t) {
	for (let n = 0; n <= items.length; n++) {
		const c = clipAt(items, t);
		const trim = c?.item.trim;
		if (!trim || c.off >= c.item.length) return t;
		if (c.off < trim.in) return c.item.start + trim.in;
		if (c.off < trim.out) return t;
		t = c.item.start + c.item.length;
	}
	return t;
}
