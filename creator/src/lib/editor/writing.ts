// The writing page's reading of a text (lap 2 C14): what each line is, the Markdown marks in it, the paragraphs for
// Compare, the words and the reading time, and the edits the selection's pill makes. Pure – test/writing.test.ts. The
// text's own rules (a Kanon text's sections) stay in @scutoons/content; this only says how a line is shown.

/** what a line of a text is on the page */
export type LineKind = 'blank' | 'heading' | 'speaker' | 'direction' | 'quote' | 'item' | 'text';

export interface LineInfo {
	kind: LineKind;
	/** the characters of the Markdown mark at the line's start (`## `, `> `, `- `), shown quiet */
	mark: number;
}

const HEADING = /^#{1,6} +(?=\S)/;
const QUOTE = /^> ?/;
const ITEM = /^(?:[-*+]|\d+[.)]) +/;
/** a name in capitals on its own line, an optional tag like (V.O.) after it – the screenplay's character line */
const SPEAKER = /^\s*(?=[^(]*[A-ZÄÖÜ][^(]*[A-ZÄÖÜ])[A-ZÄÖÜ0-9 .'’-]{2,40}(?: *\([^)]*\))?\s*$/;
/** a line in parentheses on its own – a direction */
const DIRECTION = /^\s*\(.*\)\s*$/;

export function lineInfo(line: string): LineInfo {
	if (!line.trim()) return { kind: 'blank', mark: 0 };
	const h = HEADING.exec(line);
	if (h) return { kind: 'heading', mark: h[0].length };
	const q = QUOTE.exec(line);
	if (q) return { kind: 'quote', mark: q[0].length };
	const i = ITEM.exec(line);
	if (i) return { kind: 'item', mark: i[0].length };
	if (DIRECTION.test(line)) return { kind: 'direction', mark: 0 };
	if (SPEAKER.test(line)) return { kind: 'speaker', mark: 0 };
	return { kind: 'text', mark: 0 };
}

/** a Markdown mark inside a line: `**strong**`, `*em*` / `_em_`, a link's `[words](address)` */
export interface InlineMark {
	kind: 'strong' | 'em' | 'link';
	/** the whole span, its marks included */
	from: number;
	to: number;
	/** the words inside the marks */
	inner: [number, number];
}

const INLINE = /(\*\*(?=\S)(.+?)(?<=\S)\*\*)|(\*(?=[^\s*])(.+?)(?<=[^\s*])\*)|(\b_(?=\S)(.+?)(?<=\S)_\b)|(\[([^\]\n]+)\]\(([^)\s]*)\))/g;

export function inlineMarks(line: string): InlineMark[] {
	const out: InlineMark[] = [];
	for (const match of line.matchAll(INLINE)) {
		const from = match.index ?? 0;
		const to = from + match[0].length;
		if (match[1]) out.push({ kind: 'strong', from, to, inner: [from + 2, to - 2] });
		else if (match[3] || match[5]) out.push({ kind: 'em', from, to, inner: [from + 1, to - 1] });
		else if (match[7]) out.push({ kind: 'link', from, to, inner: [from + 1, from + 1 + match[8].length] });
	}
	return out;
}

/** a line's words as pieces to show (read-only: the source in Compare) – the marks left out */
export interface Piece {
	text: string;
	strong?: boolean;
	em?: boolean;
	link?: boolean;
}

export function pieces(line: string): Piece[] {
	const info = lineInfo(line);
	const body = line.slice(info.mark);
	const out: Piece[] = [];
	let at = 0;
	for (const mark of inlineMarks(body)) {
		if (mark.from > at) out.push({ text: body.slice(at, mark.from) });
		const text = body.slice(mark.inner[0], mark.inner[1]);
		out.push(mark.kind === 'strong' ? { text, strong: true } : mark.kind === 'em' ? { text, em: true } : { text, link: true });
		at = mark.to;
	}
	if (at < body.length) out.push({ text: body.slice(at) });
	return out;
}

/** the paragraphs of a text: runs of lines between blank lines (Compare sets them side by side) */
export function paragraphs(text: string): string[] {
	return text
		.replace(/\r\n?/g, '\n')
		.split(/\n[ \t]*\n+/)
		.map((p) => p.replace(/^\n+|\n+$/g, ''))
		.filter((p, i, all) => p.trim() !== '' || (i > 0 && i < all.length - 1));
}

/** paragraphs back into one text; empty ones at the end (not written yet) leave nothing behind */
export function joinParagraphs(list: readonly string[]): string {
	const kept = [...list];
	while (kept.length && !kept[kept.length - 1].trim()) kept.pop();
	return kept.join('\n\n');
}

export function wordCount(text: string): number {
	return text.match(/[\p{L}\p{N}][\p{L}\p{N}'’-]*/gu)?.length ?? 0;
}

/** minutes to read at an unhurried pace; at least one once there is a word */
export function readingMinutes(words: number, perMinute = 200): number {
	return words ? Math.max(1, Math.round(words / perMinute)) : 0;
}

/** one edit of the text and the selection after it (offsets into the text before the edit, the selection into the text after) */
export interface WritingEdit {
	changes: { from: number; to: number; insert: string }[];
	selection: { anchor: number; head: number };
}

/** `**` or `*` around the selection, or taken off when it already has them */
export function toggleWrap(text: string, from: number, to: number, mark: string): WritingEdit {
	const n = mark.length;
	const inside = text.slice(from, to);
	if (inside.length >= 2 * n && inside.startsWith(mark) && inside.endsWith(mark) && !(n === 1 && inside.startsWith('**') && !inside.startsWith('***'))) {
		return { changes: [{ from, to, insert: inside.slice(n, inside.length - n) }], selection: { anchor: from, head: to - 2 * n } };
	}
	if (text.slice(from - n, from) === mark && text.slice(to, to + n) === mark && !(n === 1 && text.slice(from - 2, from) === '**' && text.slice(from - 3, from) !== '***')) {
		return { changes: [{ from: from - n, to: from, insert: '' }, { from: to, to: to + n, insert: '' }], selection: { anchor: from - n, head: to - n } };
	}
	return { changes: [{ from, to: from, insert: mark }, { from: to, to, insert: mark }], selection: { anchor: from + n, head: to + n } };
}

/** the lines the selection touches: [start of the first, end of the last] */
function lineSpan(text: string, from: number, to: number): [number, number] {
	const start = text.lastIndexOf('\n', from - 1) + 1;
	const endAt = text.indexOf('\n', Math.max(from, to - (to > from && text[to - 1] === '\n' ? 1 : 0)));
	return [start, endAt === -1 ? text.length : endAt];
}

/** every line the selection touches gets `prefix` (`## `, `> `), or loses it when all of them have it */
export function toggleLinePrefix(text: string, from: number, to: number, prefix: string, other: RegExp = /^(?:#{1,6} +|> ?)/): WritingEdit {
	const [start, end] = lineSpan(text, from, to);
	const lines = text.slice(start, end).split('\n');
	const all = lines.every((l) => !l.trim() || l.startsWith(prefix));
	const next = lines.map((l) => (!l.trim() ? l : all ? l.slice(prefix.length) : prefix + l.replace(other, ''))).join('\n');
	return { changes: [{ from: start, to: end, insert: next }], selection: { anchor: start, head: start + next.length } };
}

/** the lines the selection touches as a speaker: a name in capitals on its own line */
export function toSpeaker(text: string, from: number, to: number, locale?: string): WritingEdit {
	const [start, end] = lineSpan(text, from, to);
	const next = text
		.slice(start, end)
		.split('\n')
		.map((l) => l.replace(/^(?:#{1,6} +|> ?)/, '').toLocaleUpperCase(locale))
		.join('\n');
	return { changes: [{ from: start, to: end, insert: next }], selection: { anchor: start, head: start + next.length } };
}

/** the lines the selection touches in parentheses (a direction), or out of them again */
export function toggleDirection(text: string, from: number, to: number): WritingEdit {
	const [start, end] = lineSpan(text, from, to);
	const lines = text.slice(start, end).split('\n');
	const all = lines.every((l) => !l.trim() || DIRECTION.test(l));
	const next = lines
		.map((l) => {
			if (!l.trim()) return l;
			if (all) return l.replace(/^(\s*)\((.*)\)(\s*)$/, '$1$2$3');
			return `(${l.trim()})`;
		})
		.join('\n');
	return { changes: [{ from: start, to: end, insert: next }], selection: { anchor: start, head: start + next.length } };
}

/** the selection as a link's words, the caret where its address goes */
export function toLink(text: string, from: number, to: number): WritingEdit {
	const words = text.slice(from, to);
	const insert = `[${words}]()`;
	const caret = from + words.length + 3;
	return { changes: [{ from, to, insert }], selection: { anchor: caret, head: caret } };
}

/** what the selection already is (the pill shows those as on) */
export function selectionState(text: string, from: number, to: number): { strong: boolean; em: boolean; heading: boolean; quote: boolean; speaker: boolean; direction: boolean } {
	const [start, end] = lineSpan(text, from, to);
	const kinds = text
		.slice(start, end)
		.split('\n')
		.filter((l) => l.trim())
		.map((l) => lineInfo(l).kind);
	const every = (k: LineKind) => kinds.length > 0 && kinds.every((x) => x === k);
	const inside = text.slice(from, to);
	const around = (mark: string) => (inside.startsWith(mark) && inside.endsWith(mark) && inside.length >= 2 * mark.length) || (text.slice(from - mark.length, from) === mark && text.slice(to, to + mark.length) === mark);
	const strong = around('**');
	return { strong, em: !strong && around('*'), heading: every('heading'), quote: every('quote'), speaker: every('speaker'), direction: every('direction') };
}
