// Expressions in texts (agents/EXPRESSIONS.md §1): ink's inline syntax – a text is literal runs and `{…}` constructs:
// `{x}` print, `{cond: a|b}` conditional, `{a|b}` / `{&…}` / `{~…}` / `{!…}` lists. Parsed the way ink's InnerLogic does:
// an explicit list mark (`& ~ ! $`) → a list; an expression followed by `:` → a conditional; two or more `|`-separated
// elements → a sequence; otherwise the whole inside must be one expression → print. Expressions: a Pratt parser with
// ink's operator table (`&& || and or` 1, comparisons 2, `+` 4, `-` 5, `*` 6, `/` 7, `% mod` 8, prefix `- ! not`).
// Never throws: problems come back with codes and spans; bounded by EXPR_LIMITS (no input can hang it).
import { EXPR_LIMITS as L, type BinaryOp, type Content, type Expr, type ExprProblem, type ListKind, type ParsedText, type TextNode } from './ast.ts';

interface Tok {
	k: 'num' | 'str' | 'id' | 'op' | 'p' | 'eof';
	v: string;
	start: number;
	end: number;
}

interface Fail {
	fail: true;
	code: string;
	start: number;
	end: number;
	args?: string[];
}

const fail = (code: string, start: number, end: number, args?: string[]): never => {
	throw { fail: true, code, start, end, args } satisfies Fail;
};
const isFail = (e: unknown): e is Fail => !!e && (e as Fail).fail === true;

const ID = /[\p{L}_][\p{L}\p{N}_]*/uy;
const NUM = /\d+(?:\.\d+)?/y;
const WS = /[ \t\r\n]*/y;
const OPS2 = ['==', '!=', '<=', '>=', '&&', '||'];
const OPS1 = '<>+-*/%!';
const PUNCT = '(),:|}{';
const WORD_OPS = ['and', 'or', 'not', 'mod'];
const PREC: Record<string, number> = { '||': 1, or: 1, '&&': 1, and: 1, '==': 2, '!=': 2, '<': 2, '<=': 2, '>': 2, '>=': 2, '+': 4, '-': 5, '*': 6, '/': 7, '%': 8, mod: 8 };
const NORM: Record<string, BinaryOp> = { or: '||', and: '&&', mod: '%' };

const skipWs = (src: string, i: number): number => {
	WS.lastIndex = i;
	WS.test(src);
	return WS.lastIndex;
};

const match = (re: RegExp, src: string, i: number): string | null => {
	re.lastIndex = i;
	const m = re.exec(src);
	return m ? m[0] : null;
};

/** the next token at `i` (whitespace skipped) */
function lex(src: string, i: number): Tok {
	i = skipWs(src, i);
	const c = src[i];
	if (c === undefined) return { k: 'eof', v: '', start: i, end: i };
	let m = match(NUM, src, i);
	if (m) return { k: 'num', v: m, start: i, end: i + m.length };
	m = match(ID, src, i);
	if (m) {
		let end = i + m.length;
		// a dotted name: `map.total` – the dot only joins two identifiers, no spaces
		while (src[end] === '.' && (m = match(ID, src, end + 1))) end += 1 + m.length;
		const v = src.slice(i, end);
		return { k: WORD_OPS.includes(v) ? 'op' : 'id', v, start: i, end };
	}
	if (c === '"') {
		let v = '';
		for (let j = i + 1; j < src.length; j++) {
			const ch = src[j];
			if (ch === '"') return { k: 'str', v, start: i, end: j + 1 };
			if (ch === '\\' && j + 1 < src.length) v += src[++j];
			else v += ch;
		}
		return fail('unterminated-string', i, src.length);
	}
	const two = src.slice(i, i + 2);
	if (OPS2.includes(two)) return { k: 'op', v: two, start: i, end: i + 2 };
	if (OPS1.includes(c)) return { k: 'op', v: c, start: i, end: i + 1 };
	if (PUNCT.includes(c)) return { k: 'p', v: c, start: i, end: i + 1 };
	if (c === '=') return fail('single-equals', i, i + 1);
	if (c === '?' || c === '^') return fail('unsupported-operator', i, i + 1, [c]);
	const ch = String.fromCodePoint(src.codePointAt(i) ?? 0);
	return fail('unexpected-char', i, i + ch.length, [ch]);
}

/** one expression starting at `start` (stops before the first token that can't continue it); throws a Fail */
function exprAt(src: string, start: number): { expr: Expr; end: number } {
	let pos = start;
	let peeked: Tok | null = null;
	let count = 0;
	const from = skipWs(src, start);
	const peek = (): Tok => (peeked ??= lex(src, pos));
	const next = (): Tok => {
		const t = peek();
		if (t.end - from > L.length) fail('too-long', from, t.end);
		pos = t.end;
		peeked = null;
		return t;
	};
	const node = <T extends Expr>(n: T): T => {
		if (++count > L.nodes) fail('too-many-nodes', n.start, n.end);
		return n;
	};
	const deep = (d: number) => {
		if (d > L.depth) fail('too-deep', peek().start, peek().end);
	};
	const text = (t: Tok) => src.slice(t.start, t.end);

	const expr = (min: number, d: number): Expr => {
		deep(d);
		let left = unary(d);
		for (;;) {
			const t = peek();
			const p = t.k === 'op' ? PREC[t.v] : 0;
			if (!p || p <= min) return left;
			next();
			const right = expr(p, d + 1);
			left = node({ type: 'binary', op: NORM[t.v] ?? (t.v as BinaryOp), left, right, start: left.start, end: right.end });
		}
	};
	const unary = (d: number): Expr => {
		const t = peek();
		if (t.k === 'op' && (t.v === '-' || t.v === '!' || t.v === 'not')) {
			deep(d + 1);
			next();
			const arg = unary(d + 1);
			return node({ type: 'unary', op: t.v === '-' ? '-' : '!', arg, start: t.start, end: arg.end });
		}
		return primary(d);
	};
	const primary = (d: number): Expr => {
		const t = next();
		if (t.k === 'num') return node({ type: 'num', value: Number(t.v), start: t.start, end: t.end });
		if (t.k === 'str') return node({ type: 'str', value: t.v, start: t.start, end: t.end });
		if (t.k === 'id') {
			if (t.v === 'true' || t.v === 'false') return node({ type: 'bool', value: t.v === 'true', start: t.start, end: t.end });
			if (peek().v === '(' && peek().k === 'p') {
				next();
				const args: Expr[] = [];
				let close = peek();
				if (close.k === 'p' && close.v === ')') next();
				else
					for (;;) {
						deep(d + 1);
						args.push(expr(0, d + 1));
						close = next();
						if (close.k === 'p' && close.v === ',') continue;
						if (close.k === 'p' && close.v === ')') break;
						fail(close.k === 'eof' || close.k === 'p' ? 'missing-paren' : 'unexpected', close.start, close.end, [text(close)]);
					}
				return node({ type: 'call', name: t.v, args, start: t.start, end: close.end });
			}
			return node({ type: 'name', path: t.v.split('.'), start: t.start, end: t.end });
		}
		if (t.k === 'p' && t.v === '(') {
			deep(d + 1);
			const inner = expr(0, d + 1);
			const close = next();
			if (close.k !== 'p' || close.v !== ')') fail(close.k === 'eof' || close.k === 'p' ? 'missing-paren' : 'unexpected', close.start, close.end, [text(close)]);
			return inner;
		}
		if (t.k === 'eof' || (t.k === 'p' && t.v !== '{')) return fail('expected-value', t.start, t.end);
		return fail('unexpected', t.start, t.end, [text(t)]);
	};

	const e = expr(0, 1);
	return { expr: e, end: pos };
}

const problemOf = (f: Fail): ExprProblem => ({ code: f.code, severity: 'error', start: f.start, end: f.end, ...(f.args ? { args: f.args } : {}) });

/** a whole source as one expression (event conditions, the builder): `expr` null when it doesn't parse */
export function parseExpression(source: string): { expr: Expr | null; problems: ExprProblem[] } {
	const src = String(source ?? '');
	try {
		const { expr, end } = exprAt(src, 0);
		const t = lex(src, end);
		if (t.k !== 'eof') fail('unexpected', t.start, t.end, [src.slice(t.start, t.end)]);
		return { expr, problems: [] };
	} catch (e) {
		if (isFail(e)) return { expr: null, problems: [problemOf(e)] };
		throw e;
	}
}

const KINDS: Record<string, ListKind> = { $: 'sequence', '&': 'cycle', '~': 'shuffle', '!': 'once', '~!': 'shuffle-once', '!~': 'shuffle-once', '~$': 'shuffle-stopping', '$~': 'shuffle-stopping' };

/** a text → its literal runs and `{…}` constructs (escapes: `\{` `\}` `\|` `\\` – a backslash keeps the next character) */
export function parseText(input: string): ParsedText {
	const text = String(input ?? '');
	const n = text.length;
	const problems: ExprProblem[] = [];
	let constructs = 0;
	let lists = 0;
	let i = 0;
	// at most 100 problems a text (a text of 10 000 `}` doesn't need 10 000 warnings)
	const problem = (code: string, start: number, end: number, args?: string[], severity: 'error' | 'warning' = 'error') =>
		problems.length < 100 && problems.push({ code, severity, start, end, ...(args ? { args } : {}) });
	if (n > L.text && text.includes('{')) {
		problem('text-too-long', 0, n, [String(L.text)]);
		return { nodes: [{ type: 'bad', start: 0, end: n }], problems, constructs: 0, lists: 0 };
	}

	/** literal text and constructs up to the end – or, inside a construct, up to its `|` / `}` (not consumed) */
	const content = (inside: boolean, depth: number): Content => {
		const start = i;
		const nodes: TextNode[] = [];
		let buf = '';
		let from = i;
		const flush = () => {
			if (buf) nodes.push({ type: 'text', value: buf, start: from, end: i });
			buf = '';
		};
		while (i < n) {
			const c = text[i];
			if (!buf) from = i;
			if (c === '\\') {
				buf += i + 1 < n ? text[i + 1] : '\\';
				i += 2;
				if (i > n) i = n;
			} else if (c === '{') {
				flush();
				nodes.push(construct(depth));
			} else if (c === '}' && !inside) {
				problem('stray-close', i, i + 1, undefined, 'warning');
				buf += c;
				i++;
			} else if ((c === '}' || c === '|') && inside) break;
			else {
				buf += c;
				i++;
			}
		}
		flush();
		return { start, end: i, nodes };
	};

	/** the elements of a list from `i`: `a|b|c}` – `closed` when its `}` was there (then consumed) */
	const items = (depth: number): { items: Content[]; closed: boolean } => {
		const all = [content(true, depth + 1)];
		while (text[i] === '|') {
			i++;
			all.push(content(true, depth + 1));
		}
		const closed = text[i] === '}';
		if (closed) i++;
		return { items: all, closed };
	};

	/** past the `}` that closes the construct whose inside starts at `from` (n when there is none) */
	const skipToClose = (from: number): number => {
		let level = 0;
		for (let j = from; j < n; j++) {
			const c = text[j];
			if (c === '\\') j++;
			else if (c === '{') level++;
			else if (c === '}' && level-- === 0) return j + 1;
		}
		return n;
	};

	const bad = (start: number): TextNode => ({ type: 'bad', start, end: i });

	/** one `{…}` at `i` */
	const construct = (depth: number): TextNode => {
		const s = i;
		if (++constructs > L.constructs) {
			problem('too-many-constructs', s, n, [String(L.constructs)]);
			i = n;
			return bad(s);
		}
		if (depth >= L.depth) {
			i = skipToClose(s + 1);
			problem('too-deep', s, i, [String(L.depth)]);
			return bad(s);
		}
		i = skipWs(text, s + 1);
		let mark = '';
		while ('!&~$'.includes(text[i] ?? 'x') && mark.length < 3) mark += text[i++];
		if (mark) {
			const kind = KINDS[mark];
			const index = lists++;
			const r = items(depth);
			if (!r.closed) {
				problem('missing-close', s, n);
				return bad(s);
			}
			if (!kind) {
				problem('unsupported-list', s, s + 1 + mark.length, [mark]);
				return bad(s);
			}
			return { type: 'list', kind, index, items: r.items, start: s, end: i };
		}
		const p = i;
		let attempt: { expr: Expr; end: number } | null = null;
		let failure: Fail | null = null;
		try {
			attempt = exprAt(text, p);
		} catch (e) {
			if (!isFail(e)) throw e;
			failure = e;
		}
		const q = attempt ? skipWs(text, attempt.end) : -1;
		if (attempt && text[q] === ':') {
			i = q + 1;
			const then = content(true, depth + 1);
			let otherwise: Content | null = null;
			if (text[i] === '|') {
				i++;
				otherwise = content(true, depth + 1);
				if (text[i] === '|') {
					problem('conditional-too-many', i, i + 1);
					while (text[i] === '|') {
						i++;
						content(true, depth + 1);
					}
					if (text[i] === '}') i++;
					else problem('missing-close', s, n);
					return bad(s);
				}
			}
			if (text[i] !== '}') {
				problem('missing-close', s, n);
				return bad(s);
			}
			i++;
			return { type: 'cond', cond: attempt.expr, then, else: otherwise, start: s, end: i };
		}
		// a sequence when there are two or more elements – otherwise its parse is thrown away again
		const before = { problems: problems.length, constructs, lists };
		const index = lists++;
		i = p;
		const r = items(depth);
		if (r.closed && r.items.length >= 2) return { type: 'list', kind: 'sequence', index, items: r.items, start: s, end: i };
		problems.length = before.problems;
		constructs = before.constructs;
		lists = before.lists;
		if (attempt && text[q] === '}') {
			i = q + 1;
			return { type: 'print', expr: attempt.expr, start: s, end: i };
		}
		if (!r.closed) problem('missing-close', s, n);
		else if (!text.slice(s + 1, i - 1).trim()) problem('empty-braces', s, i);
		else if (failure) problem(failure.code, failure.start, failure.end, failure.args);
		else {
			try {
				const t = lex(text, q);
				problem('unexpected', t.start, t.end, [text.slice(t.start, t.end)]);
			} catch (e) {
				if (!isFail(e)) throw e;
				problem(e.code, e.start, e.end, e.args);
			}
		}
		return bad(s);
	};

	const top = content(false, 0);
	return { nodes: top.nodes, problems, constructs, lists };
}
