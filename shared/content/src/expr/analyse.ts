// Expressions in texts (agents/EXPRESSIONS.md §4): the static analyser for the creator, validation and the flow graph –
// problems with positions and stable codes (English messages here; the creator can translate by `code` + `args`), and what
// a text reads: names, functions with their literal arguments, how many lists. Ported to PHP
// (server/src/Domain/Content/Expr/Analyser.php) – both are tested against test/fixtures/expr/*.json.
import type { Content, Expr, ExprProblem, TextNode } from './ast.ts';
import { parseExpression, parseText } from './parse.ts';
import { EXPR_FUNCTIONS, EXPR_NAMESPACES, type ExprFunction, type ExprNamespace, type ExprParam } from './registry.ts';

export interface ExprRegistry {
	functions: readonly ExprFunction[];
	namespaces: readonly ExprNamespace[];
}

export interface ExprDiagnostic extends ExprProblem {
	message: string;
}

export interface ExprReads {
	/** names read, each once, in order: `['coins']`, `['map', 'total']` */
	names: string[][];
	/** functions called, each name + literal arguments once: an argument that isn't a literal is null */
	functions: { name: string; literalArgs: (string | number | boolean | null)[] }[];
	/** how many lists (sequences / cycles / shuffles / once-only) */
	sequences: number;
}

export interface ExprAnalysis {
	errors: ExprDiagnostic[];
	reads: ExprReads;
	constructs: number;
}

export interface AnalyseOptions {
	registry?: ExprRegistry;
	/** the story variables there are (bare names): an unknown one becomes a warning with a suggestion */
	knownNames?: readonly string[];
}

const MESSAGES: Record<string, string> = {
	'missing-close': 'missing `}`',
	'missing-paren': 'missing `)`',
	'empty-braces': 'empty `{}` – to show a brace, write `\\{`',
	unexpected: 'unexpected `{0}`',
	'unexpected-char': 'unexpected character `{0}`',
	'expected-value': 'expected a value',
	'single-equals': '`=` doesn’t compare – write `==`',
	'unsupported-operator': '`{0}` isn’t supported (ink lists aren’t)',
	'unterminated-string': 'missing closing `"`',
	'unsupported-list': '`{0}` isn’t a list type – use `&` (cycle), `~` (shuffle), `!` (once only), `~!` or `~$`',
	'conditional-too-many': 'conditional text has one `|` at most: `{condition: then|else}`',
	'stray-close': 'unmatched `}` – to show it, write `\\}`',
	'too-long': 'expression too long (more than 500 characters)',
	'too-deep': 'nested too deeply (more than 32 levels)',
	'too-many-nodes': 'expression too complex (more than 64 parts)',
	'too-many-constructs': 'too many `{…}` in one text (more than {0})',
	'text-too-long': 'text too long for `{…}` (more than {0} characters)',
	'unknown-name': 'unknown name `{0}`',
	'unknown-variable': 'unknown variable `{0}`',
	'unknown-function': 'unknown function `{0}`',
	'function-needs-call': '`{0}` is a function – write `{0}(…)`',
	'not-a-function': '`{0}` isn’t a function – write `{0}` without `(…)`',
	'namespace-needs-member': '`{0}` needs a name after it, e.g. `{1}`',
	'argument-count': '`{0}` expects {1}',
	'argument-type': '`{0}` expects a {1} as argument {2}',
	'old-clock-syntax': '`{{0}:clock}` is the old form – write `{clock({0})}`',
	'or-in-list': '`||` here splits a list (as in ink) – for “or”, write `or`',
	'not-in-list': '`{!…}` is a once-only list (as in ink) – for “not”, write `not`',
	'missing-value': '`{0}` has no value',
	'not-a-number': '`{0}` needs numbers',
	'compare-text': '`{0}` can’t compare texts – only `==` and `!=` can',
	'division-by-zero': 'division by zero',
	'bad-arguments': '`{0}` can’t use these arguments',
};

/** a problem's English message ("unknown name `map.totl` – did you mean `map.total`?") */
export function exprMessage(p: Pick<ExprProblem, 'code' | 'args' | 'suggestion'>): string {
	const args = p.args ?? [];
	const text = (MESSAGES[p.code] ?? p.code).replace(/\{(\d)\}/g, (_, i: string) => args[Number(i)] ?? '');
	return p.suggestion ? `${text} – did you mean \`${p.suggestion}\`?` : text;
}

/** the edit distance of two words (Levenshtein) */
export const editDistance = (a: string, b: string): number => distance(a, b);
const distance = (a: string, b: string): number => {
	let prev = Array.from({ length: b.length + 1 }, (_, j) => j);
	for (let i = 1; i <= a.length; i++) {
		const row = [i];
		for (let j = 1; j <= b.length; j++) row[j] = Math.min(prev[j] + 1, row[j - 1] + 1, prev[j - 1] + (a[i - 1] === b[j - 1] ? 0 : 1));
		prev = row;
	}
	return prev[b.length];
};

/** the closest candidate, when it's close enough (at most 2 edits and a third of the word – none for words under 3 letters) */
export function didYouMean(word: string, candidates: Iterable<string>): string | undefined {
	let best: string | undefined;
	let bestD = Infinity;
	const limit = Math.min(2, Math.floor(word.length / 3));
	if (!limit) return undefined;
	for (const c of candidates) {
		if (Math.abs(c.length - word.length) > limit) continue;
		const d = distance(word.toLowerCase(), c.toLowerCase());
		if (d <= limit && d < bestD) {
			best = c;
			bestD = d;
		}
	}
	return best;
}

const plural = (n: number, variadic?: boolean) => `${variadic ? 'at least ' : ''}${n} argument${n === 1 ? '' : 's'}`;

class Analyser {
	errors: ExprDiagnostic[] = [];
	names = new Map<string, string[]>();
	functions = new Map<string, ExprReads['functions'][number]>();
	fns: Map<string, ExprFunction>;
	spaces: Map<string, ExprNamespace>;
	known: Set<string> | null;

	constructor(
		opts: AnalyseOptions,
		readonly src = '',
	) {
		const registry = opts.registry ?? { functions: EXPR_FUNCTIONS, namespaces: EXPR_NAMESPACES };
		this.fns = new Map(registry.functions.map((f) => [f.name, f]));
		this.spaces = new Map(registry.namespaces.map((ns) => [ns.name, ns]));
		this.known = opts.knownNames ? new Set(opts.knownNames) : null;
	}

	add(p: ExprProblem) {
		this.errors.push({ ...p, message: exprMessage(p) });
	}

	problem(code: string, at: { start: number; end: number }, args: string[], suggestion?: string, severity: 'error' | 'warning' = 'error') {
		this.add({ code, severity, start: at.start, end: at.end, args, ...(suggestion ? { suggestion } : {}) });
	}

	/** every namespace member as `ns.member`, values or functions */
	paths(kind: 'value' | 'function'): string[] {
		return [...this.spaces.values()].flatMap((ns) => ns.members.filter((m) => m.kind === kind).map((m) => `${ns.name}.${m.name}`));
	}

	member(dotted: string) {
		const dot = dotted.indexOf('.');
		if (dot < 0) return { ns: undefined, m: undefined };
		const ns = this.spaces.get(dotted.slice(0, dot));
		return { ns, m: ns?.members.find((m) => m.name === dotted.slice(dot + 1)) };
	}

	expr(e: Expr): void {
		switch (e.type) {
			case 'name':
				return this.name(e);
			case 'call':
				e.args.forEach((a) => this.expr(a));
				return this.call(e);
			case 'unary':
				return this.expr(e.arg);
			case 'binary':
				this.expr(e.left);
				return this.expr(e.right);
		}
	}

	name(e: Expr & { type: 'name' }) {
		const full = e.path.join('.');
		if (e.path.length === 1) {
			const ns = this.spaces.get(full);
			if (ns) return this.problem('namespace-needs-member', e, [full, `${full}.${ns.members[0]?.name ?? 'name'}`]);
			if (this.fns.has(full)) return this.problem('function-needs-call', e, [full]);
			if (this.known && !this.known.has(full)) this.problem('unknown-variable', e, [full], didYouMean(full, this.known), 'warning');
		} else {
			const { ns, m } = this.member(full);
			if (m?.kind === 'function') return this.problem('function-needs-call', e, [full]);
			if (!m) return this.problem('unknown-name', e, [full], didYouMean(full, ns ? ns.members.filter((x) => x.kind === 'value').map((x) => `${ns.name}.${x.name}`) : this.paths('value')));
		}
		if (!this.names.has(full)) this.names.set(full, e.path);
	}

	call(e: Expr & { type: 'call' }) {
		let params: ExprParam[] | undefined;
		let variadic = false;
		const fn = this.fns.get(e.name);
		if (fn) {
			params = fn.params;
			variadic = !!fn.variadic;
		} else {
			const { ns, m } = this.member(e.name);
			if (m?.kind === 'value') return this.problem('not-a-function', e, [e.name]);
			if (!m) {
				const all = ns ? ns.members.filter((x) => x.kind === 'function').map((x) => `${ns.name}.${x.name}`) : [...this.fns.keys(), ...this.paths('function')];
				return this.problem('unknown-function', e, [e.name], didYouMean(e.name, all));
			}
			params = m.params ?? [];
		}
		const count = e.args.length;
		if (variadic ? count < params.length : count !== params.length) return this.problem('argument-count', e, [e.name, plural(params.length, variadic)]);
		e.args.forEach((arg, i) => {
			const type = (params[i] ?? params[params.length - 1])?.type;
			if (type === 'number' && (arg.type === 'str' || arg.type === 'bool')) this.problem('argument-type', arg, [e.name, 'number', String(i + 1)]);
		});
		const literalArgs = e.args.map((a) => (a.type === 'num' || a.type === 'str' || a.type === 'bool' ? a.value : null));
		const key = JSON.stringify([e.name, literalArgs]);
		if (!this.functions.has(key)) this.functions.set(key, { name: e.name, literalArgs });
	}

	content(c: Content | null) {
		c?.nodes.forEach((n) => this.node(n));
	}

	node(n: TextNode) {
		switch (n.type) {
			case 'print':
				return this.expr(n.expr);
			case 'cond': {
				this.expr(n.cond);
				const only = n.then.nodes[0];
				if (n.cond.type === 'name' && !n.else && n.then.nodes.length === 1 && only.type === 'text' && only.value === 'clock')
					this.problem('old-clock-syntax', n, [n.cond.path.join('.')], undefined, 'warning');
				this.content(n.then);
				return this.content(n.else);
			}
			case 'list':
				if (n.kind === 'sequence' && n.items.some((c) => !c.nodes.length && this.src.slice(c.start - 1, c.start + 1) === '||'))
					this.problem('or-in-list', n, [], undefined, 'warning');
				if (n.kind === 'once' && n.items.length === 1) this.problem('not-in-list', n, [], undefined, 'warning');
				return n.items.forEach((c) => this.content(c));
		}
	}

	result(constructs: number, sequences: number): ExprAnalysis {
		return { errors: this.errors, reads: { names: [...this.names.values()], functions: [...this.functions.values()], sequences }, constructs };
	}
}

/** a text's problems (sorted by position) and reads */
export function analyseText(text: string, opts: AnalyseOptions = {}): ExprAnalysis {
	const parsed = parseText(text);
	const a = new Analyser(opts, String(text ?? ''));
	parsed.problems.forEach((p) => a.add(p));
	parsed.nodes.forEach((n) => a.node(n));
	a.errors.sort((x, y) => x.start - y.start || x.end - y.end);
	return a.result(parsed.constructs, parsed.lists);
}

/** one expression's problems and reads (event conditions) */
export function analyseExpression(source: string, opts: AnalyseOptions = {}): ExprAnalysis {
	const parsed = parseExpression(source);
	const a = new Analyser(opts);
	parsed.problems.forEach((p) => a.add(p));
	if (parsed.expr) a.expr(parsed.expr);
	return a.result(0, 0);
}
