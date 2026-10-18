import assert from 'node:assert/strict';
import { test } from 'node:test';
import { anchorScorePiece, pageGeometry, placeScorePiece, type PageGeometry } from '../src/score/score-anchors.ts';
import { barIndex, type BarRef } from '../src/score/score-bars.ts';
import { scoreOptions } from '../src/score/score-layout.ts';
import type { ScoreToolkit } from '../src/score/verovio.ts';

// One file, laid out by the real Verovio 6.3.0 at the reader's sizes (the same load, as the score view re-lays it):
// a piece on a note stays on that note whether its bar starts a line in one layout and sits mid-line in the other
// (agents/research/COLLAGE.md §4.2). The music is written here – two parts, 12 bars of 4/4 in B flat: quarters,
// eighths, a dotted rhythm, sixteenths, a triplet, a grace note, chords, rests, a bar rest, a second voice, lyrics.

const DIV = 12;
const LENGTHS: Record<string, [number, string, boolean?]> = { w: [48, 'whole'], h: [24, 'half'], 'q.': [18, 'quarter', true], q: [12, 'quarter'], e: [6, 'eighth'], s: [3, '16th'], t: [4, 'eighth'], 't[': [4, 'eighth'], 't]': [4, 'eighth'] };
const WORDS = ['la', 'shine', 'o', 'twinkling', 'a', 'stars'];
let word = 0;

const pitch = (p: string) => {
	const [, step, acc, octave] = /^([A-G])([#b]?)(\d)$/.exec(p)!;
	return `<pitch><step>${step}</step>${acc ? `<alter>${acc === '#' ? 1 : -1}</alter>` : ''}<octave>${octave}</octave></pitch>`;
};

/** "C5 q" a note, "C5+E5 h" a chord, "r q" a rest, "R" the bar's rest, "~G5" a grace note, "A5 t[" … "G5 t]" a triplet */
function note(token: string, voice: number, lyrics: boolean): string {
	if (token === 'R') return `<note><rest measure="yes"/><duration>${4 * DIV}</duration><voice>${voice}</voice></note>`;
	if (token.startsWith('~')) return `<note><grace/>${pitch(token.slice(1))}<voice>${voice}</voice><type>eighth</type></note>`;
	const [what, length] = token.split(' ');
	const [duration, type, dot] = LENGTHS[length];
	const triplet = length.startsWith('t') ? '<time-modification><actual-notes>3</actual-notes><normal-notes>2</normal-notes></time-modification>' : '';
	const tail = `<duration>${duration}</duration><voice>${voice}</voice><type>${type}</type>${dot ? '<dot/>' : ''}${triplet}`;
	const bracket = length === 't[' ? 'start' : length === 't]' ? 'stop' : '';
	const notations = bracket ? `<notations><tuplet type="${bracket}" bracket="yes"/></notations>` : '';
	if (what === 'r') return `<note><rest/>${tail}</note>`;
	const lyric = lyrics ? `<lyric><syllabic>single</syllabic><text>${WORDS[word++ % WORDS.length]}</text></lyric>` : '';
	return what.split('+').map((p, i) => `<note>${i ? '<chord/>' : ''}${pitch(p)}${tail}${notations}${i ? '' : lyric}</note>`).join('');
}

function part(id: string, clef: string, bars: string[], lyrics: boolean): string {
	const measures = bars.map((bar, i) => {
		const [one, two] = bar.split(' | ');
		const voices = note2(one, 1, lyrics) + (two ? `<backup><duration>${4 * DIV}</duration></backup>${note2(two, 2, false)}` : '');
		const attributes = i ? '' : `<attributes><divisions>${DIV}</divisions><key><fifths>-2</fifths></key><time><beats>4</beats><beat-type>4</beat-type></time><clef>${clef}</clef></attributes>`;
		return `<measure number="${i + 1}">${attributes}${voices}</measure>`;
	});
	return `<part id="${id}">${measures.join('')}</part>`;
}
const note2 = (voice: string, n: number, lyrics: boolean) => voice.split(', ').map((token) => note(token, n, lyrics)).join('');

const XML = `<?xml version="1.0" encoding="UTF-8"?><score-partwise version="3.1"><part-list>
<score-part id="P1"><part-name>Voice</part-name></score-part><score-part id="P2"><part-name>Bass</part-name></score-part></part-list>
${part('P1', '<sign>G</sign><line>2</line>', ['C5 q, D5 q, E5 q, F5 q', 'G5 q., F5 e, E5 h', 'D5 e, E5 e, F5 e, G5 e, A5 q, r q', 'Bb5 t[, A5 t, G5 t], F5 q, E5 h', '~G5, F5 q, E5 q, D5 h', 'C5 w', 'R', 'E5 s, F5 s, G5 s, A5 s, Bb5 q, A5 h', 'G5 q, F#5 q, G5 h', 'C5+E5 h, D5+F5 h', 'E5 q, D5 q, C5 q, r q', 'F5 w'], true)}
${part('P2', '<sign>F</sign><line>4</line>', ['C3 h, G3 h', 'C3 w | E3 q, F3 q, G3 h', 'F3 q, A3 q, C4 q, A3 q', 'Bb2 h, F3 h', 'C3 q, E3 q, G3 q, E3 q', 'F2 w', 'C3 e, D3 e, E3 e, F3 e, G3 h', 'R', 'D3 h, G2 h', 'C3+G3 w', 'G2 q, A2 q, B2 q, r q', 'F2 w'], false)}
</score-partwise>`;

interface Layout {
	name: string;
	pages: PageGeometry[];
	/** a note's notehead: its left edge (viewBox x) and its page */
	heads: Map<string, { x: number; page: number; bar: string }>;
}

const HEAD = /<g id="([^"]+)" class="note"[^>]*>\s*<g class="notehead">\s*<use [^>]*transform="translate\((-?[\d.]+)/g;

async function toolkit(): Promise<ScoreToolkit> {
	const [{ default: createModule }, { VerovioToolkit, enableLog, LOG_ERROR }] = await Promise.all([import('verovio/wasm'), import('verovio/esm')]);
	const module = await createModule();
	// a phone line justified wide warns on every layout – the test's output keeps errors only
	enableLog(LOG_ERROR, module);
	return new VerovioToolkit(module) as ScoreToolkit;
}

function layout(tk: ScoreToolkit, bars: BarRef[], name: string, w: number, h: number, zoom: number): Layout {
	tk.resetOptions();
	tk.setOptions(scoreOptions(w, h, zoom, null, 16));
	tk.redoLayout();
	const pages: PageGeometry[] = [];
	const heads = new Map<string, { x: number; page: number; bar: string }>();
	for (let p = 1; p <= tk.getPageCount(); p++) {
		const svg = tk.renderToSVG(p);
		const geo = pageGeometry(svg, bars)!;
		pages[p] = geo;
		const shift = Number(/class="page-margin" transform="translate\((-?[\d.]+)/.exec(svg)![1]);
		const starts = [...svg.matchAll(/<g id="([^"]+)" class="measure"/g)].map((m) => ({ id: m[1], at: m.index! }));
		for (const m of svg.matchAll(HEAD)) {
			const bar = starts.filter((s) => s.at < m.index!).pop()!.id;
			heads.set(m[1], { x: Number(m[2]) + shift, page: p, bar });
		}
	}
	return { name, pages, heads };
}

/** the bars that start a line in this layout */
const lineStarts = (l: Layout) => new Set(l.pages.flatMap((geo) => geo?.bars.filter((bar, i) => i === 0 || geo.bars[i - 1].staves[0].top !== bar.staves[0].top).map((bar) => bar.n) ?? []));

test('a piece on a note stays on it across layouts – also when its bar starts a line in one and sits mid-line in the other', async () => {
	const tk = await toolkit();
	tk.setOptions(scoreOptions(390, 700, 30, null, 16));
	assert.ok(tk.loadData(XML), 'Verovio takes the file');
	const bars = barIndex(tk.getMEI());
	assert.equal(bars.length, 12);
	assert.ok(bars.every((bar) => bar.onsets!.size > 0), 'every bar has its time');
	const layouts = [layout(tk, bars, 'phone', 390, 700, 30), layout(tk, bars, 'phone zoom +', 390, 700, 35), layout(tk, bars, 'desktop', 1280, 700, 40)];
	const [phone, , desktop] = layouts;
	// the case of the review: bars that start a line on the phone and sit mid-line on the desktop
	const flipped = [...lineStarts(phone)].filter((n) => n !== '1' && !lineStarts(desktop).has(n));
	assert.ok(flipped.length > 0, `some bar starts a line on the phone only (phone ${[...lineStarts(phone)]}, desktop ${[...lineStarts(desktop)]})`);
	let checked = 0;
	for (const from of layouts) {
		for (const to of layouts) {
			if (from === to) continue;
			for (const bar of bars) {
				for (const id of bar.onsets!.keys()) {
					const a = from.heads.get(id);
					const b = to.heads.get(id);
					if (!a || !b) continue; // a chord's id (its notes are checked)
					const geoA = from.pages[a.page];
					const geoB = to.pages[b.page];
					const boxA = geoA.bars.find((box) => box.id === a.bar)!;
					// on the note: half a staff space into its notehead, a space below the top staff's top line
					const point = { x: ((a.x + 0.5 * geoA.sp) / geoA.width) * 100, y: ((boxA.staves[0].top + geoA.sp) / geoA.height) * 100 };
					const anchored = anchorScorePiece(point, geoA)!;
					assert.equal(anchored.at.bar, bar.n, `${from.name}: the note's bar`);
					const place = placeScorePiece({ at: anchored.at, rect: { x: anchored.x, y: anchored.y, width: 4 } }, geoB)!;
					const off = ((place.left / 100) * geoB.width - (b.x + 0.5 * geoB.sp)) / geoB.sp;
					assert.ok(Math.abs(off) <= 0.5, `bar ${bar.n} note ${id}: ${from.name} → ${to.name} ${off.toFixed(2)} sp off`);
					checked++;
				}
			}
		}
	}
	assert.ok(checked > 300, `${checked} placements`);
	// the old meaning (% of the staff lines) on a flipped bar's first note: off by the clef, key and time signature
	const n = flipped[0];
	const first = [...bars.find((bar) => bar.n === n)!.onsets!].find(([id, t]) => t === 0 && phone.heads.has(id))![0];
	const [a, b] = [desktop.heads.get(first)!, phone.heads.get(first)!];
	const [boxA, boxB] = [desktop.pages[a.page].bars.find((box) => box.id === a.bar)!, phone.pages[b.page].bars.find((box) => box.id === b.bar)!];
	const share = (a.x - boxA.left) / (boxA.right - boxA.left);
	const old = (boxB.left + share * (boxB.right - boxB.left) - b.x) / phone.pages[b.page].sp;
	assert.ok(Math.abs(old) > 3, `the staff lines' % would put it ${old.toFixed(1)} sp off`);
});
