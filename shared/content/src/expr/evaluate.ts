// Expressions in texts (agents/EXPRESSIONS.md §1): evaluation – pure, a walk over the parsed text with the reader's
// answers from a resolver. Never throws to the caller and readers never see raw code: a construct that can't be
// evaluated renders nothing and its problem comes back as a diagnostic. Values work the ink way: `+` with a text joins
// texts, `==` with a text compares texts, numbers / booleans compare as numbers, `and` / `or` short-circuit and give a
// boolean, booleans print as true / false. Missing values (`undefined`) print as nothing, are false, compare equal only
// to another missing value, and make arithmetic missing too.
import { EXPR_LIMITS as L, type Content, type Expr, type ExprProblem, type ExprValue, type ParsedText, type Span, type TextNode } from './ast.ts';
import { parseExpression, parseText } from './parse.ts';
import { EXPR_IMPLS } from './registry.ts';

/** what the reader answers – a SCUTOONS resolver against the fixed namespaces (EXPR_NAMESPACES), never generic access */
export interface ExprResolver {
	/** a name's value: `['coins']`, `['map', 'total']` – undefined when there is none */
	name(path: readonly string[]): unknown;
	/** a state function (`has`, `count`, `inventory.has` …): undefined when there is no answer */
	call?(name: string, args: readonly ExprValue[]): unknown;
	/** how many times the text's list `index` was shown BEFORE this showing (0 the first time) */
	visit?(index: number): number;
	/** shuffles and random(): fixed per seed (reader + text) */
	seed?: string | number;
	/** the text's language (BCP 47) for number() / upper() … */
	language?: string;
}

export interface TextResult {
	text: string;
	problems: ExprProblem[];
	/** the lists shown this time (their `index`): the reader adds one visit to each */
	visited: number[];
}

interface RunFail {
	run: true;
	problem: ExprProblem;
}

const stop = (code: string, at: Span, args?: string[]): never => {
	throw { run: true, problem: { code, severity: 'error', start: at.start, end: at.end, ...(args ? { args } : {}) } } satisfies RunFail;
};

/** a value as text, the ink way: whole numbers plain, fractions without float noise, booleans true / false */
export function showValue(value: ExprValue): string {
	if (value === undefined) return '';
	if (typeof value === 'number') return Number.isInteger(value) ? String(value) : String(Number.parseFloat(value.toPrecision(12)));
	return String(value);
}

const clean = (value: unknown): ExprValue => {
	if (typeof value === 'string') return value.length > L.string ? value.slice(0, L.string) : value;
	if (typeof value === 'number') return Number.isFinite(value) ? value : undefined;
	if (typeof value === 'boolean') return value;
	return undefined;
};

/** a 32-bit hash of a text (FNV-1a) */
const hash = (text: string): number => {
	let h = 0x811c9dc5;
	for (let i = 0; i < text.length; i++) h = Math.imul(h ^ text.charCodeAt(i), 0x01000193);
	return h >>> 0;
};

/** a number in [0, 1) from a key (mulberry32 step) */
const unit = (key: string): number => {
	let t = (hash(key) + 0x6d2b79f5) | 0;
	t = Math.imul(t ^ (t >>> 15), t | 1);
	t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
	return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
};

const truthy = (v: ExprValue): boolean => (typeof v === 'string' ? v.length > 0 : !!v);

const ask = <T>(fn: () => T, fallback: T): T => {
	try {
		return fn();
	} catch {
		return fallback;
	}
};

const ARITH = '+-*/%';

/** one evaluation for a resolver (closures, not a class: they minify better in the reader) */
function runner(resolver: ExprResolver) {
	const problems: ExprProblem[] = [];
	const visited: number[] = [];
	const language = resolver.language || 'en';
	const seed = resolver.seed ?? '';

	const missing = (e: Span, what: string): undefined => {
		problems.push({ code: 'missing-value', severity: 'warning', start: e.start, end: e.end, args: [what] });
		return undefined;
	};

	const value = (e: Expr): ExprValue => {
		switch (e.type) {
			case 'num':
			case 'str':
			case 'bool':
				return e.value;
			case 'name': {
				const got = clean(ask(() => resolver.name(e.path), undefined));
				return got === undefined ? missing(e, e.path.join('.')) : got;
			}
			case 'call': {
				const args = e.args.map(value);
				const impl = Object.hasOwn(EXPR_IMPLS, e.name) ? EXPR_IMPLS[e.name] : null;
				if (impl) {
					const result = clean(ask(() => impl(args, { language, random: () => unit(`${seed}:${e.start}`), show: showValue }), undefined));
					return result === undefined ? stop('bad-arguments', e, [e.name]) : result;
				}
				const got = clean(ask(() => resolver.call?.(e.name, args), undefined));
				return got === undefined ? missing(e, `${e.name}(…)`) : got;
			}
			case 'unary': {
				const x = value(e.arg);
				if (e.op === '!') return !truthy(x);
				if (x === undefined) return undefined;
				return typeof x === 'string' ? stop('not-a-number', e, ['-']) : -Number(x);
			}
			case 'binary':
				return binary(e);
		}
	};

	const binary = (e: Expr & { type: 'binary' }): ExprValue => {
		const op = e.op;
		if (op === '&&') return truthy(value(e.left)) && truthy(value(e.right));
		if (op === '||') return truthy(value(e.left)) || truthy(value(e.right));
		const a = value(e.left);
		const b = value(e.right);
		const text = typeof a === 'string' || typeof b === 'string';
		const arith = ARITH.includes(op);
		if (op === '==' || op === '!=') {
			const same = a === undefined || b === undefined ? a === b : text ? showValue(a) === showValue(b) : Number(a) === Number(b);
			return op === '==' ? same : !same;
		}
		if (op === '+' && text) return (showValue(a) + showValue(b)).slice(0, L.string);
		if (text) return stop(arith ? 'not-a-number' : 'compare-text', e, [op]);
		if (a === undefined || b === undefined) return arith ? undefined : false;
		const x = Number(a);
		const y = Number(b);
		if (op === '<') return x < y;
		if (op === '<=') return x <= y;
		if (op === '>') return x > y;
		if (op === '>=') return x >= y;
		if ((op === '/' || op === '%') && y === 0) return stop('division-by-zero', e);
		const r = op === '+' ? x + y : op === '-' ? x - y : op === '*' ? x * y : op === '/' ? x / y : x % y;
		return Number.isFinite(r) ? r : stop('not-a-number', e, [op]);
	};

	/** an expression's value – ok false plus a problem when it can't be evaluated */
	const guarded = (e: Expr): { ok: boolean; value: ExprValue } => {
		try {
			return { ok: true, value: value(e) };
		} catch (x) {
			if (!(x as RunFail)?.run) throw x;
			problems.push((x as RunFail).problem);
			return { ok: false, value: undefined };
		}
	};

	const content = (c: Content | null): string => (c ? c.nodes.map(node).join('') : '');

	/** ink's shuffle: each round of `k` visits shows every element once, in an order fixed by seed + round */
	const shuffled = (index: number, count: number, k: number): number => {
		const order = Array.from({ length: k }, (_, i) => i);
		const round = Math.floor(count / k);
		for (let i = k - 1; i > 0; i--) {
			const j = Math.floor(unit(`${seed}:${index}:${round}:${i}`) * (i + 1));
			[order[i], order[j]] = [order[j], order[i]];
		}
		return order[count % k];
	};

	const node = (n: TextNode): string => {
		if (n.type === 'text') return n.value;
		if (n.type === 'print') {
			const r = guarded(n.expr);
			return r.ok ? showValue(r.value) : '';
		}
		if (n.type === 'cond') {
			const r = guarded(n.cond);
			return !r.ok ? '' : truthy(r.value) ? content(n.then) : content(n.else);
		}
		if (n.type !== 'list') return '';
		const count = Math.max(0, Math.floor(Number(ask(() => resolver.visit?.(n.index), 0)) || 0));
		visited.push(n.index);
		const len = n.items.length;
		const kind = n.kind;
		const pick =
			kind === 'sequence'
				? Math.min(count, len - 1)
				: kind === 'cycle'
					? count % len
					: kind === 'once'
						? count < len
							? count
							: -1
						: kind === 'shuffle'
							? shuffled(n.index, count, len)
							: kind === 'shuffle-once'
								? count < len
									? shuffled(n.index, count, len)
									: -1
								: count < len - 1
									? shuffled(n.index, count, len - 1)
									: len - 1;
		return pick < 0 ? '' : content(n.items[pick]);
	};

	return { problems, visited, node, guarded };
}

/** ink's output clean-up: runs of spaces collapse to one, no spaces at the start / end of a line */
const tidy = (text: string): string =>
	text
		.split('\n')
		.map((line) => line.replace(/[ \t]+/g, ' ').replace(/^ | $/g, ''))
		.join('\n');

const cache = new Map<string, ParsedText>();

/** a text, parsed once (a small cache – texts repeat while reading) */
export function parsedText(text: string): ParsedText {
	let parsed = cache.get(text);
	if (!parsed) {
		parsed = parseText(text);
		if (cache.size >= 256) cache.delete(cache.keys().next().value!);
		cache.set(text, parsed);
	}
	return parsed;
}

/** a text with its `{…}` evaluated for this reader; a text without `{ } \` comes back as it is */
export function evaluateText(text: string, resolver: ExprResolver): TextResult {
	const source = String(text ?? '');
	if (!/[{}\\]/.test(source)) return { text: source, problems: [], visited: [] };
	const parsed = parsedText(source);
	const run = runner(resolver);
	const out = parsed.nodes.map(run.node).join('');
	return {
		text: parsed.constructs ? tidy(out) : out,
		problems: [...parsed.problems.filter((p) => p.severity === 'error'), ...run.problems],
		visited: run.visited,
	};
}

/** one expression's value (event conditions): undefined plus problems when it doesn't parse / evaluate */
export function evaluateExpression(source: string, resolver: ExprResolver): { value: ExprValue; problems: ExprProblem[] } {
	const parsed = parseExpression(source);
	if (!parsed.expr) return { value: undefined, problems: parsed.problems };
	const run = runner(resolver);
	const r = run.guarded(parsed.expr);
	return { value: r.value, problems: run.problems };
}

/** is a value true the ink way (0, "", false, missing are false) */
export const isTruthy = truthy;
