/**
 * The file's bars, as the collage pieces name them (agents/research/COLLAGE.md §4.2): from the MEI the toolkit holds
 * (`barIndex(tk.getMEI())`, one pass per load – the ids are the SVG's, and stay through `redoLayout`), every measure's
 * id, its number as the file writes it, which one of that number – and when its notes and rests start, the time a
 * piece's x runs in (score-anchors.ts: a piece on a note stays on that note whatever the line breaks do).
 *
 * The time comes from the lengths Verovio's MusicXML import writes on every note, chord, rest and space (`dur.ppq`, in
 * pulses of the staff's `staffDef@ppq` per quarter): a layer's events follow one another, a chord's notes start with
 * the chord, grace notes take no time; the bar lasts as long as its longest layer. An event without a length stops
 * that layer's count (what came before stays). A bar rest (mRest, a repeat sign …) is drawn in the bar's middle:
 * `rests`, not `onsets`. On a page (`beatsOf`, from the glyphs score-anchors.ts reads) each moment stands in the middle
 * of its leftmost notehead and the barline ends the bar; `barX` / `barTime` go between the bar's time and page x.
 * Pure and node-tested (test/score-anchors.test.ts, test/score-reflow.test.ts).
 */
import type { ScoreAt } from '@scutoons/content';

/** a bar of the file: its id in the toolkit's MEI / SVG, its number as the file writes it, the n-th bar of that number */
export interface BarRef {
	id: string;
	n: string;
	nth: number;
	/** when its notes and rests start (their MEI / SVG id → part of the bar: 0 = its first beat, 1 = its end) */
	onsets?: ReadonlyMap<string, number>;
	/** its bar rests' ids (mRest, multiRest, mRpt …): one glyph in the middle of the bar, whatever its length */
	rests?: ReadonlySet<string>;
}

const ATTRS = new Map<string, RegExp>();
export const attr = (attrs: string, name: string): string | null => {
	let re = ATTRS.get(name);
	if (!re) ATTRS.set(name, (re = new RegExp(`(?:^|\\s)${name.replace(/\./g, '\\.')}="([^"]*)"`)));
	const m = re.exec(attrs);
	return m ? m[1] : null;
};

/** an XML tag: closing slash, name, attributes (quoted values may hold anything), self-closing slash */
export const TAG = /<(\/?)([A-Za-z][\w:.-]*)((?:[^>"']|"[^"]*"|'[^']*')*?)(\/?)>/g;

const BAR_RESTS = new Set(['mRest', 'multiRest', 'mRpt', 'mRpt2', 'multiRpt']);
const EVENTS = new Set(['note', 'chord', 'rest', 'space']);

/** the bars of the file in its order (`tk.getMEI()`): every measure's id, number, which one of that number, its time */
export function barIndex(mei: string): BarRef[] {
	const bars: BarRef[] = [];
	const seen = new Map<string, number>();
	/** staff n → pulses per quarter (the latest staffDef) */
	const ppq = new Map<string, number>();
	let bar: { onsets: Map<string, number>; rests: Set<string>; length: number } | null = null;
	let pulse = 1;
	/** the open layer's time in quarters; NaN outside a layer and after an event without a length */
	let clock = NaN;
	/** the open chord's onset (NaN: a grace chord, or a chord the layer can't time) */
	let chord: number | null = null;
	let grace = 0;
	for (const m of mei.matchAll(TAG)) {
		const [, closing, name, attrs, selfClosing] = m;
		if (closing) {
			if (name === 'layer' && bar) {
				if (!Number.isNaN(clock)) bar.length = Math.max(bar.length, clock);
				clock = NaN;
			} else if (name === 'chord') chord = null;
			else if (name === 'graceGrp') grace = Math.max(0, grace - 1);
			else if (name === 'measure' && bar) {
				settle(bar);
				bar = null;
			}
			continue;
		}
		const id = attr(attrs, 'xml:id');
		if (name === 'staffDef') {
			const pulses = Number(attr(attrs, 'ppq'));
			if (pulses > 0) ppq.set(attr(attrs, 'n') ?? '', pulses);
		} else if (name === 'measure') {
			if (!id) continue;
			const n = attr(attrs, 'n') ?? '';
			const nth = (seen.get(n) ?? 0) + 1;
			seen.set(n, nth);
			const ref = { id, n, nth, onsets: new Map<string, number>(), rests: new Set<string>() };
			bars.push(ref);
			bar = selfClosing ? null : { onsets: ref.onsets, rests: ref.rests, length: 0 };
		} else if (!bar) continue;
		else if (name === 'staff') pulse = ppq.get(attr(attrs, 'n') ?? '') ?? 1;
		else if (name === 'layer') clock = selfClosing ? NaN : 0;
		else if (name === 'graceGrp') grace += selfClosing ? 0 : 1;
		else if (name === 'note' && chord !== null) {
			// a chord's note starts with the chord (and takes no time of its own)
			if (id && !Number.isNaN(chord)) bar.onsets.set(id, chord);
		} else if (Number.isNaN(clock)) continue;
		else if (BAR_RESTS.has(name)) {
			if (id) bar.rests.add(id);
		} else if (EVENTS.has(name)) {
			const isGrace = grace > 0 || attr(attrs, 'grace') !== null;
			if (name === 'chord' && !selfClosing) chord = isGrace ? NaN : clock;
			if (isGrace) continue;
			if (id && name !== 'space') bar.onsets.set(id, clock);
			const length = Number(attr(attrs, 'dur.ppq')) / pulse;
			clock = length > 0 ? clock + length : NaN;
		}
	}
	return bars;
}

/** a bar's onsets from quarters into parts of the bar; a bar of no length has none */
function settle(bar: { onsets: Map<string, number>; length: number }) {
	if (!(bar.length > 0)) bar.onsets.clear();
	for (const [id, at] of bar.onsets) bar.onsets.set(id, Math.min(at / bar.length, 1));
}

/** the file's bar a piece names (`at.bar`, the `nth` of that number – default 1); none = the file has no such bar */
export function findBar<T extends BarRef>(bars: readonly T[], at: Pick<ScoreAt, 'bar' | 'nth'> | null | undefined): T | undefined {
	if (!at || typeof at.bar !== 'string') return undefined;
	const n = at.bar.trim();
	const nth = at.nth ?? 1;
	return bars.find((bar) => bar.n === n && bar.nth === nth);
}

// ── a bar's time on a rendered page (score-anchors.ts reads the glyphs; a piece's x runs through these) ──

/** a moment of a bar on the page: `t` = when (0 = the bar's first beat, 1 = its end), `x` = where (viewBox units) */
export interface Beat {
	t: number;
	x: number;
}

/** a note / rest drawn in a bar: when it starts, its glyph's left edge, a bar rest */
export interface Drawn {
	t: number;
	x: number;
	rest: boolean;
}

/**
 * half a notehead in staff spaces (SMuFL's black notehead is 1.18–1.3 sp wide, a rest about as wide): a beat stands
 * in the middle of its note, where a piece centred on that note sits – so that piece stays on it in every layout
 */
const HALF_HEAD = 0.6;

/**
 * A bar's beats from its drawn events: one per moment, half a notehead right of its leftmost glyph (the staves share
 * Verovio's columns; a second's displaced head or a voice pushed aside stays the same distance off in every layout);
 * bar rests only when nothing else is drawn; a moment left of the one before it (a time the MEI got wrong) is left
 * out, so t and x rise. `left` / `right` = the bar's staff lines, `sp` = the page's staff space (viewBox units).
 */
export function beatsOf(drawn: readonly Drawn[], left: number, right: number, sp: number): Beat[] {
	const head = HALF_HEAD * sp;
	const timed = drawn.some((d) => !d.rest) ? drawn.filter((d) => !d.rest) : drawn;
	const at = new Map<number, number>();
	for (const { t, x } of timed) {
		const key = Math.round(t * 1e6) / 1e6;
		at.set(key, Math.min(at.get(key) ?? Infinity, x + head));
	}
	const beats: Beat[] = [];
	for (const [t, x] of [...at].sort((a, b) => a[0] - b[0])) if (t < 1 && x < right && (!beats.length || x > beats[beats.length - 1].x)) beats.push({ t, x });
	if (!beats.length) beats.push({ t: 0, x: left });
	beats.push({ t: 1, x: right });
	return beats;
}

/** the bar's mean pace (x per t, first beat to barline) – how its time goes on before the first note and after the barline */
const pace = (beats: readonly Beat[]) => (beats[beats.length - 1].x - beats[0].x) / (beats[beats.length - 1].t - beats[0].t);

/** where the bar's time `t` stands on the page (viewBox x): linear between its beats, at its mean pace outside them */
export function barX(bar: { beats: readonly Beat[] }, t: number): number {
	const { beats } = bar;
	const first = beats[0];
	const last = beats[beats.length - 1];
	if (t <= first.t) return first.x + (t - first.t) * pace(beats);
	if (t >= last.t) return last.x + (t - last.t) * pace(beats);
	let i = 1;
	while (beats[i].t < t) i++;
	const [a, b] = [beats[i - 1], beats[i]];
	return a.x + ((t - a.t) / (b.t - a.t)) * (b.x - a.x);
}

/** the bar's time at page x (viewBox units) – barX's inverse */
export function barTime(bar: { beats: readonly Beat[] }, x: number): number {
	const { beats } = bar;
	const first = beats[0];
	const last = beats[beats.length - 1];
	const p = pace(beats);
	if (!(p > 0)) return 0;
	if (x <= first.x) return first.t + (x - first.x) / p;
	if (x >= last.x) return last.t + (x - last.x) / p;
	let i = 1;
	while (beats[i].x < x) i++;
	const [a, b] = [beats[i - 1], beats[i]];
	return a.t + ((x - a.x) / (b.x - a.x)) * (b.t - a.t);
}
