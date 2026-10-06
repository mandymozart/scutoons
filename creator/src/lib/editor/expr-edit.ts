// Expressions in the creator's text fields (agents/EXPRESSIONS.md §4 + step 4): what a field shows over its plain text –
// the top-level `{…}` constructs as CHIPS, the analyser's problems as marks with their level – and the builder's model:
// a chip's code read back into "show a value" / "if … then … else …" / a list, and written again as ink syntax. Pure
// (no DOM, no messages): the field (components/fields/expr-text.svelte) and the builder (expr-builder.svelte) use it,
// tests in test/expr-edit.test.ts. The stored value stays the plain text – chips are display only.
import { analyseExpression, analyseText, exprLevel, parseText, type Content, type Expr, type ExprDiagnostic, type ListKind } from '@scutoons/content';

/** a top-level `{…}` of the text (closed with its `}`): shown as one chip */
export interface Chip {
	start: number;
	end: number;
	/** print / conditional / list – `bad` = closed but doesn't parse (a chip with its error) */
	kind: 'print' | 'cond' | 'list' | 'bad';
}

/** an analyser problem at its place in the field's text (paragraph offsets added) with its validation level */
export interface Mark {
	start: number;
	end: number;
	level: 'error' | 'warning';
	code: string;
	args?: string[];
	suggestion?: string;
	/** English (exprMessage) – the field words it in the creator's language by `code` */
	message: string;
}

export interface FieldView {
	chips: Chip[];
	marks: Mark[];
}

/**
 * The parts a text is analysed in: the whole text, or – a text box's lines (`string[]`, shown with a blank line between
 * them, locale-text.svelte) – each paragraph on its own, as validation checks them (each line its own text).
 */
export function textParts(text: string, paragraphs = false): { start: number; text: string }[] {
	if (!paragraphs) return [{ start: 0, text }];
	const out: { start: number; text: string }[] = [];
	const gap = /\n{2,}/g;
	let from = 0;
	for (let m = gap.exec(text); m; m = gap.exec(text)) {
		out.push({ start: from, text: text.slice(from, m.index) });
		from = m.index + m[0].length;
	}
	out.push({ start: from, text: text.slice(from) });
	return out;
}

/** the offset in the field of a stored line's offset (`line` of a string[] text, shown joined by blank lines) */
export function fieldOffset(text: string, line: number | undefined, offset: number, paragraphs = false): number {
	if (line === undefined || !paragraphs) return offset;
	const part = textParts(text, true)[line];
	return part ? part.start + offset : offset;
}

const toMark = (p: ExprDiagnostic, at: number): Mark => ({
	start: p.start + at,
	end: Math.max(p.end, p.start + 1) + at,
	level: exprLevel(p) === 'error' ? 'error' : 'warning',
	code: p.code,
	...(p.args ? { args: p.args } : {}),
	...(p.suggestion ? { suggestion: p.suggestion } : {}),
	message: p.message
});

/** a text field's chips + marks (`knownNames` = the toon's story variables: an unknown one warns with a suggestion) */
export function textView(text: string, opts: { knownNames?: readonly string[]; paragraphs?: boolean } = {}): FieldView {
	const chips: Chip[] = [];
	const marks: Mark[] = [];
	if (!/[{}\\]/.test(text)) return { chips, marks };
	for (const part of textParts(text, opts.paragraphs)) {
		if (!/[{}\\]/.test(part.text)) continue;
		for (const node of parseText(part.text).nodes) {
			if (node.type === 'text') continue;
			if (node.type === 'bad' && part.text[node.end - 1] !== '}') continue;
			chips.push({ start: node.start + part.start, end: node.end + part.start, kind: node.type });
		}
		for (const p of analyseText(part.text, { knownNames: opts.knownNames }).errors) marks.push(toMark(p, part.start));
	}
	return { chips, marks };
}

/** an expression field (a condition: the whole value is code, no braces): its marks */
export function expressionView(source: string, knownNames?: readonly string[]): FieldView {
	if (!source.trim()) return { chips: [], marks: [] };
	return { chips: [], marks: analyseExpression(source, { knownNames }).errors.map((p) => toMark(p, 0)) };
}

/** the chip the offset is strictly inside (between its braces' outer edges) */
export const chipInside = (chips: readonly Chip[], at: number) => chips.find((c) => c.start < at && at < c.end);
/** the chip that ends / starts exactly at the offset */
export const chipEndingAt = (chips: readonly Chip[], at: number) => chips.find((c) => c.end === at);
export const chipStartingAt = (chips: readonly Chip[], at: number) => chips.find((c) => c.start === at);
/** marks touching a range, the worst first */
export const marksIn = (marks: readonly Mark[], start: number, end: number) =>
	marks.filter((m) => m.start < end && m.end > start).sort((a, b) => (a.level === b.level ? a.start - b.start : a.level === 'error' ? -1 : 1));

/** a caret moved into a (not edited) chip snaps to the edge it came from: from the left → its end, else its start */
export function snapCaret(chips: readonly Chip[], at: number, from: number): number {
	const chip = chipInside(chips, at);
	if (!chip) return at;
	return from <= chip.start ? chip.end : from >= chip.end ? chip.start : at - chip.start < chip.end - at ? chip.start : chip.end;
}

/** a selection grows to whole chips (copying / deleting a part of a chip takes all of it) */
export function growSelection(chips: readonly Chip[], start: number, end: number): [number, number] {
	const a = chipInside(chips, start);
	const b = chipInside(chips, end);
	return [a ? a.start : start, b ? b.end : end];
}

// ── the builder's model ──────────────────────────────────────────────────────────────────────────────────────────────

/** what the builder offers between two values (ink's operators; `mod` = `%`) */
export const BUILDER_OPS = ['==', '!=', '<', '<=', '>', '>=', '+', '-', '*', '/', '%'] as const;
export type BuilderOp = (typeof BUILDER_OPS)[number];
/** the list kinds the builder writes (shuffle once / stopping stay code) */
export const BUILDER_LISTS = ['sequence', 'cycle', 'shuffle', 'once'] as const;
export type BuilderList = (typeof BUILDER_LISTS)[number];
const LIST_MARK: Record<BuilderList, string> = { sequence: '', cycle: '&', shuffle: '~', once: '!' };

export type BuilderKind = 'value' | 'cond' | 'list' | 'code';

export interface BuilderModel {
	kind: BuilderKind;
	/** value / cond: the first value (a name, a call, a literal – any expression) */
	left: string;
	/** value / cond: the operator with a second value ('' = none) */
	op: BuilderOp | '';
	right: string;
	/** cond: the texts */
	then: string;
	else: string;
	/** list: its kind + items */
	list: BuilderList;
	items: string[];
	/** code: the construct as written (text fields: with its braces; a condition: the expression) */
	code: string;
}

export const emptyModel = (kind: BuilderKind = 'value'): BuilderModel => ({ kind, left: '', op: '', right: '', then: '', else: '', list: 'sequence', items: ['', ''], code: '' });

const slice = (src: string, span: { start: number; end: number }) => src.slice(span.start, span.end);

/** an expression as value + operator + value when its top is one of the builder's operators, else one value */
function splitExpr(src: string, expr: Expr): Pick<BuilderModel, 'left' | 'op' | 'right'> {
	if (expr.type === 'binary' && (BUILDER_OPS as readonly string[]).includes(expr.op)) {
		return { left: slice(src, expr.left).trim(), op: expr.op as BuilderOp, right: slice(src, expr.right).trim() };
	}
	return { left: slice(src, expr).trim(), op: '', right: '' };
}

/**
 * A chip's code (`{…}`) – or a condition's expression (`expression`) – as the builder's model: one value (with an
 * operator), a conditional, a list; anything else (shuffle once, nested constructs the builder can't show, syntax
 * errors) opens as code.
 */
export function modelOf(code: string, expression = false): BuilderModel {
	const model = emptyModel();
	if (!code.trim()) return model;
	if (expression) {
		const parsed = parseTextExpression(code);
		return parsed ? { ...model, ...parsed } : { ...model, kind: 'code', code };
	}
	const parsed = parseText(code);
	const node = parsed.nodes.length === 1 ? parsed.nodes[0] : null;
	if (!node || parsed.problems.some((p) => p.severity === 'error') || node.start !== 0 || node.end !== code.length) return { ...model, kind: 'code', code };
	if (node.type === 'print') return { ...model, ...splitExpr(code, node.expr) };
	if (node.type === 'cond') {
		// the space after `:` is the builder's own (it writes `{cond: then|else}`)
		const flat = (c: Content | null) => (c ? slice(code, c) : '');
		return { ...model, kind: 'cond', ...splitExpr(code, node.cond), then: flat(node.then).replace(/^ /, ''), else: flat(node.else) };
	}
	if (node.type === 'list' && (BUILDER_LISTS as readonly string[]).includes(node.kind)) {
		return { ...model, kind: 'list', list: node.kind as BuilderList, items: node.items.map((c) => slice(code, c)) };
	}
	return { ...model, kind: 'code', code };
}

function parseTextExpression(src: string): Pick<BuilderModel, 'left' | 'op' | 'right'> | null {
	// a condition's expression is the inside of a print: `{` + src + `}` keeps the offsets one apart
	const parsed = parseText(`{${src}}`);
	const node = parsed.nodes.length === 1 ? parsed.nodes[0] : null;
	if (!node || node.type !== 'print' || parsed.problems.some((p) => p.severity === 'error')) return null;
	return splitExpr(`{${src}}`, node.expr);
}

/** the value part: `left`, or `left op right` */
const valueCode = (m: BuilderModel) => {
	const left = m.left.trim();
	const right = m.right.trim();
	return m.op && right ? `${left} ${m.op} ${right}` : left;
};

/** the model as ink syntax: a text field's construct with braces, a condition's bare expression (`expression`) */
export function codeOf(m: BuilderModel, expression = false): string {
	if (m.kind === 'code') return m.code.trim();
	if (expression) return valueCode(m);
	if (m.kind === 'value') return `{${valueCode(m)}}`;
	if (m.kind === 'cond') return `{${valueCode(m)}: ${m.then}${m.else ? `|${m.else}` : ''}}`;
	const items = m.items.length ? m.items : [''];
	// one item: a plain `{a}` would print a value – a sequence of one keeps its mark (`$` = sequence)
	const mark = LIST_MARK[m.list] || (items.length < 2 ? '$' : '');
	return `{${mark}${items.join('|')}}`;
}

/** a short readable line for a chip's tooltip: `coins == 1 ? coin | coins`, `cycle: a | b | c` */
export function chipSummary(code: string, words: { cond?: (cond: string, then: string, otherwise: string) => string; list?: (kind: ListKind, items: string[]) => string } = {}): string {
	const parsed = parseText(code);
	const node = parsed.nodes.length === 1 ? parsed.nodes[0] : null;
	if (!node) return code;
	if (node.type === 'print') return slice(code, node.expr);
	if (node.type === 'cond') {
		const cond = slice(code, node.cond);
		const then = slice(code, node.then).trim();
		const otherwise = node.else ? slice(code, node.else) : '';
		return words.cond ? words.cond(cond, then, otherwise) : `${cond} ? ${then}${otherwise ? ` | ${otherwise}` : ''}`;
	}
	if (node.type === 'list') {
		const items = node.items.map((c) => slice(code, c));
		return words.list ? words.list(node.kind, items) : `${node.kind}: ${items.join(' | ')}`;
	}
	return code;
}
