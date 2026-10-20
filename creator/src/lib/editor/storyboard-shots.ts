// The storyboard page's shots (#123): the chapter document's shots in toon order with their places, the shot list's
// picking (the studio's: a click = only that shot, Ctrl / ⌘ = in or out of the picked set, Shift = the range from the
// last clicked one – multi-select.ts clickSelect), a field set or unset on a shot, the schematic as an image. The
// model and its logic are @scutoons/content/storyboard's; nothing here copies it.
import type { ChapterStoryboard, Shot, StoryboardDoc } from '@scutoons/content/storyboard';
import { aspectSlices, FORMATS, SLICE_ASPECTS } from '@scutoons/content/storyboard';
import { clickSelect } from './multi-select.ts';

/** a shot with its place in the document (`path` = where `doc.edit` finds it) */
export interface ShotEntry {
	id: string;
	shot: Shot;
	scene: StoryboardDoc;
	sceneIndex: number;
	path: (string | number)[];
}

/** every shot of the chapter document in toon order (scenes → groups → shots) */
export function shotEntries(doc: ChapterStoryboard | null | undefined): ShotEntry[] {
	const out: ShotEntry[] = [];
	(doc?.scenes ?? []).forEach((scene, si) =>
		(scene.groups ?? []).forEach((g, gi) =>
			(g.shots ?? []).forEach((shot, i) => {
				if (shot?.id) out.push({ id: shot.id, shot, scene, sceneIndex: si, path: ['scenes', si, 'groups', gi, 'shots', i] });
			})
		)
	);
	return out;
}

/** the shot list's state: the picked shots (toon order) and the current one (the one the panels edit) */
export interface Picking {
	picked: string[];
	current: string | null;
	/** the last plain / Ctrl click (Shift spans from here) */
	anchor: string | null;
}

/**
 * A click on shot `id` in the list. The clicked shot becomes the current one; one taken off the set (Ctrl) hands the
 * current to its neighbour in the set; the last one stays picked.
 */
export function pickShot(order: readonly string[], state: Picking, id: string, event: { shiftKey?: boolean; ctrlKey?: boolean; metaKey?: boolean } = {}): Picking {
	const index = order.indexOf(id);
	if (index < 0) return state;
	const picked = new Set(state.picked);
	if (state.current) picked.add(state.current);
	const selected = order.map((x, i) => (picked.has(x) ? i : -1)).filter((i) => i >= 0);
	const anchorAt = order.indexOf(state.anchor ?? state.current ?? '');
	const toggle = event.ctrlKey || event.metaKey;
	// Ctrl + click on the only picked shot: nothing to take off
	if (toggle && !event.shiftKey && selected.length === 1 && selected[0] === index) return state;
	const next = clickSelect(selected, anchorAt >= 0 ? anchorAt : null, index, event);
	// Ctrl + Shift: the range joins the set (clickSelect alone replaces it)
	const set = toggle && event.shiftKey ? [...new Set([...selected, ...next.selected])].sort((a, b) => a - b) : next.selected;
	const ids = set.map((i) => order[i]);
	const removed = toggle && !event.shiftKey && !ids.includes(id);
	let current = id;
	if (removed) {
		// the current one went off the set: its neighbour in the set (the one after it, else the one before)
		current = state.current && ids.includes(state.current) ? state.current : (ids.find((x) => order.indexOf(x) > index) ?? ids[ids.length - 1]);
	}
	return { picked: ids, current, anchor: event.shiftKey ? state.anchor ?? state.current : order[next.anchor] };
}

const empty = (v: unknown) => v === undefined || v === null || v === '' || (typeof v === 'object' && !Array.isArray(v) && Object.keys(v as object).length === 0);

/**
 * Sets a field at `keys` inside `target` (a shot, a figure) – an empty value ('' / null / undefined / {}) removes it,
 * and an object left empty by that goes too (`light: {}`; never an item of a list). Reads the parents back through `target` after every write
 * (on Svelte $state data a `??=` result is the raw object – GOTCHAS).
 */
export function setField(target: Record<string, any>, keys: readonly string[], value: unknown): void {
	if (!keys.length) return;
	const parents: Record<string, any>[] = [target];
	for (const k of keys.slice(0, -1)) {
		const at = parents[parents.length - 1];
		if (at[k] === undefined || at[k] === null || typeof at[k] !== 'object') {
			if (empty(value)) return;
			at[k] = {};
		}
		parents.push(at[k]);
	}
	const last = keys[keys.length - 1];
	if (empty(value)) delete parents[parents.length - 1][last];
	else parents[parents.length - 1][last] = value;
	// prune the parents emptied by the removal, innermost first
	for (let i = parents.length - 1; i > 0; i--) {
		// an emptied list item stays (a hole would shift the others)
		if (!empty(parents[i]) || Array.isArray(parents[i - 1])) break;
		delete parents[i - 1][keys[i - 1]];
	}
}

/** the schematic SVG (content's shotSvg) as an image source */
export const svgSource = (svg: string) => `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;

/**
 * The words the inspector offers that content's vocabulary doesn't name (the studio's lists; validate doesn't check
 * them): the aspects (the slice grid's first), the accent colours, the personas, the moods.
 */
export const ASPECTS = [...SLICE_ASPECTS, '9:16', '1:2', '1:3', '1:4', '1:5', '1:6', '1:7', '2:1', '3:1', '4:1', '6:1'];
/** the free aspects of a panorama / a sideways pass (loop360, strip) */
export const FREE_ASPECTS = ASPECTS.filter((a) => !(SLICE_ASPECTS as readonly string[]).includes(a));

/**
 * The Aspect select's options by format (#111, research/SLICE-GRID.md §3.1 – whole slices): a viewport is one slice
 * (1:1.6), a vertical shot 2 or more (1:3.2 … 1:8), each with its count ("1:4.8 · 3 slices") – what the validator
 * accepts; a stored value outside that stays selected and says what it is ("9:16 – not whole slices", or a viewport's
 * "1:3.2 · 2 slices"), so nothing is lost; loop360 / strip keep the free aspects. `words` = the page's language.
 */
export function aspectOptions(
	format: string | undefined,
	current: string | undefined,
	words: { slices: (count: number) => string; offGrid: string }
): { value: string; label: string }[] {
	if (format !== 'viewport' && format !== 'vertical') return optionsOf(FREE_ASPECTS, current);
	const grid: readonly string[] = format === 'viewport' ? SLICE_ASPECTS.slice(0, 1) : SLICE_ASPECTS.slice(1);
	const label = (value: string) => {
		const k = aspectSlices(value);
		return k ? `${value} · ${words.slices(k)}` : `${value} – ${words.offGrid}`;
	};
	return (current && !grid.includes(current) ? [...grid, current] : grid).map((value) => ({ value, label: label(value) }));
}
export const ACCENTS = ['red', 'blue', 'green', 'amber', 'yellow'];
export const PERSONAS = ['regular', 'human', 'cat', 'mountaintiger', 'biker'];
export const MOODS = ['acceptance', 'angry', 'bargaining', 'confused', 'crying', 'denial', 'disgusted', 'dizzy', 'embarrassed', 'furious', 'happy', 'laughing', 'neutral', 'sad', 'scared', 'smug', 'surprised', 'thinking'];

/** a list as select options, the current value kept when it isn't in the list (an old word stays visible) */
export function optionsOf(list: readonly string[], current?: string | null): { value: string; label: string }[] {
	const words = current && !list.includes(current) ? [...list, current] : list;
	return words.map((value) => ({ value, label: value }));
}

/** a new figure for a shot: the scene's first cast entry, standing in the middle */
export const newFigure = (cast: string) => ({ cast, pose: 'stand', facing: 'front', framing: 'full', x: 0.5, y: 0.6, h: 0.3 });

/** the card's facts in the shot list: format + aspect, the view */
export const shotFacts = (shot: Shot): string[] => [`${FORMATS.includes(shot.format) ? shot.format : shot.format ?? '?'} ${shot.aspect ?? ''}`.trim(), shot.view ?? '–'];

/** the shot id a validate message names ("scene-25 sb-25-3: …" → sb-25-3), checked against the known ids */
export function shotOfMessage(message: string, ids: ReadonlySet<string>): string | null {
	const m = /^\S+\s+(\S+?):\s/.exec(message);
	return m && ids.has(m[1]) ? m[1] : null;
}

/** a validate message split into its place (scene, shot) and its text: "scene-25 sb-25-3: x" → ['scene-25', 'sb-25-3'], 'x' */
export function splitMessage(message: string): { place: string[]; text: string } {
	const m = /^([^:]+?):\s(.*)$/s.exec(message);
	return m ? { place: m[1].split(/\s+/), text: m[2] } : { place: [], text: message };
}
