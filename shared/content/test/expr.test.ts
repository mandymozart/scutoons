// Expressions in texts (agents/EXPRESSIONS.md): ink's inline syntax – parser, evaluator, analyser, registry, limits.
// The shared golden cases (also run by the PHP port) are in expr-golden.test.ts.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { analyseExpression, analyseText, didYouMean, exprMessage } from '../src/expr/analyse.ts';
import { EXPR_LIMITS, type ExprValue, type TextNode } from '../src/expr/ast.ts';
import { evaluateExpression, evaluateText, isTruthy, showValue, type ExprResolver } from '../src/expr/evaluate.ts';
import { parseExpression, parseText } from '../src/expr/parse.ts';
import { EXPR_FUNCTIONS, EXPR_IMPLS, EXPR_NAMESPACES } from '../src/expr/registry.ts';

const vars: Record<string, ExprValue> = { coins: 3, name: 'Pope', yes: true, no: false, time: 330, 'map.total': 12, 'map.unlocked': 10 };
const resolver = (extra: Partial<ExprResolver> = {}): ExprResolver => ({
	name: (path) => (Object.hasOwn(vars, path.join('.')) ? vars[path.join('.')] : undefined),
	call: (name, args) => (name === 'has' ? args[0] === 'cucumber' : undefined),
	seed: 1,
	language: 'en',
	...extra,
});
const run = (text: string, extra: Partial<ExprResolver> = {}) => evaluateText(text, resolver(extra)).text;
const codes = (text: string, knownNames?: string[]) => analyseText(text, knownNames ? { knownNames } : {}).errors.map((e) => e.code);

test('print: names, namespaces, literals, ink printing of numbers and booleans', () => {
	assert.equal(run('Hi {name}, {coins} coins, {map.unlocked}/{map.total}'), 'Hi Pope, 3 coins, 10/12');
	assert.equal(run('{yes} {no} {1.25} {2.0} {0.1 + 0.2} {"x"}'), 'true false 1.25 2 0.3 x');
	assert.equal(showValue(undefined), '');
	assert.equal(showValue(1 / 3), '0.333333333333');
});

test('conditionals: then / else, nesting, ink plurals', () => {
	assert.equal(run('You have {coins} {coins == 1: coin|coins}.'), 'You have 3 coins.');
	assert.equal(run('{yes: {no: a|b}|c}'), 'b');
	assert.equal(run('{no: never}x'), 'x');
	assert.equal(run('{map.unlocked + 2 == map.total: Two more to go|Long way from home}'), 'Two more to go');
	assert.equal(run('{has("cucumber"): You smell of cucumber.}'), 'You smell of cucumber.');
});

test('lists: sequence stops, cycle loops, once-only runs out, visited reported', () => {
	const at = (text: string, count: number) => run(text, { visit: () => count });
	assert.deepEqual([0, 1, 2, 5].map((c) => at('{a|b|c}', c)), ['a', 'b', 'c', 'c']);
	assert.deepEqual([0, 1, 2, 3].map((c) => at('{&a|b|c}', c)), ['a', 'b', 'c', 'a']);
	assert.deepEqual([0, 1, 2].map((c) => at('{!a|b}', c)), ['a', 'b', '']);
	const r = evaluateText('{yes: {a|b}|{c|d}} {e|f}', resolver());
	assert.deepEqual(r.visited, [0, 2], 'the else branch list (1) was not shown');
	assert.equal(parseText('{a|{b|c}} {d|e}').lists, 3);
});

test('shuffle: every element once per round of visits, stable per seed, different seeds differ', () => {
	const text = '{~a|b|c|d|e}';
	for (const seed of [1, 2, 'x']) {
		for (let round = 0; round < 3; round++) {
			const seen = new Set(Array.from({ length: 5 }, (_, i) => run(text, { seed, visit: () => round * 5 + i })));
			assert.equal(seen.size, 5, `seed ${seed} round ${round}`);
		}
	}
	assert.equal(run(text, { seed: 7, visit: () => 3 }), run(text, { seed: 7, visit: () => 3 }));
	const orders = new Set([1, 2, 3, 4, 5, 6].map((seed) => [0, 1, 2, 3, 4].map((v) => run(text, { seed, visit: () => v })).join('')));
	assert.ok(orders.size > 1);
	assert.deepEqual([0, 1, 2, 3].map((v) => run('{~!a|b}', { visit: () => v })).slice(2), ['', '']);
	assert.equal(run('{~$a|b|c}', { visit: () => 2 }), 'c');
	assert.equal(run('{~$a|b|c}', { visit: () => 99 }), 'c');
});

test('random(a, b): whole numbers in range, fixed per seed and call site', () => {
	const values = new Set<string>();
	for (let seed = 0; seed < 200; seed++) {
		const v = run('{random(1, 6)}', { seed });
		assert.match(v, /^[1-6]$/);
		values.add(v);
		assert.equal(v, run('{random(1, 6)}', { seed }));
	}
	assert.equal(values.size, 6);
	assert.equal(run('{random(6, 1)}', { seed: 3 }).length, 1, 'bounds in either order');
	assert.equal(run('{random(2, 2)}'), '2');
});

test('operators: ink precedence incl. mod, short-circuit and / or, strings vs numbers', () => {
	const val = (src: string, extra: Partial<ExprResolver> = {}) => evaluateExpression(src, resolver(extra)).value;
	assert.equal(val('1 + 2 * 3'), 7);
	assert.equal(val('2 * 7 mod 4'), 6, 'ink: mod binds tighter than *');
	assert.equal(val('10 - 4 - 3'), 3);
	assert.equal(val('7 / 2'), 3.5);
	assert.equal(val('"5" == 5'), true);
	assert.equal(val('"a" + 1 + 2'), 'a12');
	assert.equal(val('1 + 2 + "a"'), '3a');
	assert.equal(val('"b" > "a"'), undefined);
	assert.equal(val('not no and yes'), true);
	assert.equal(val('yes || no && no'), false, 'ink: && and || share one precedence, left to right');
	let asked = 0;
	const counting = { name: (p: readonly string[]) => (asked++, p[0] === 't' ? 1 : 0) };
	assert.equal(evaluateExpression('t or x', counting).value, true);
	assert.equal(asked, 1, 'or short-circuits');
	asked = 0;
	assert.equal(evaluateExpression('f and x', counting).value, false);
	assert.equal(asked, 1, 'and short-circuits');
	assert.equal(isTruthy(''), false);
	assert.equal(isTruthy('0'), true);
	assert.equal(isTruthy(0), false);
});

test('runtime problems: empty construct + diagnostic, never a throw', () => {
	const r = evaluateText('a {1 / 0} b {"x" - 1} c {nothing} d', resolver());
	assert.equal(r.text, 'a b c d');
	assert.deepEqual(
		r.problems.map((p) => p.code),
		['division-by-zero', 'not-a-number', 'missing-value'],
	);
	const throwing: ExprResolver = {
		name: () => {
			throw new Error('boom');
		},
		call: () => {
			throw new Error('boom');
		},
		visit: () => {
			throw new Error('boom');
		},
	};
	assert.equal(evaluateText('{x}{has("a")}{a|b}{upper(x)}', throwing).text, 'a');
	const odd: ExprResolver = { name: () => ({ toString: () => 'evil' }), call: () => [1, 2] };
	assert.equal(evaluateText('[{x}][{has("a")}]', odd).text, '[][]', 'objects are no values');
	const huge: ExprResolver = { name: () => 'x'.repeat(100000) };
	assert.equal(evaluateText('{x + x}', huge).text.length, EXPR_LIMITS.string);
});

test('safety: __proto__ / constructor are just names, a plain-object resolver can’t leak prototypes', () => {
	const data: Record<string, unknown> = { coins: 1 };
	const naive: ExprResolver = { name: (p) => data[p.join('.')], call: (n) => data[n] };
	assert.equal(evaluateText('{__proto__}{constructor}{toString}{hasOwnProperty("x")}{constructor.name}', naive).text, '');
	assert.equal(evaluateText('{toString(1)}{valueOf()}{__proto__(1)}', resolver()).text, '');
	assert.ok(codes('{toString(1)}').includes('unknown-function'));
	assert.ok(codes('{map.constructor}').includes('unknown-name'));
	assert.equal(Object.hasOwn(EXPR_IMPLS, 'constructor'), false);
});

test('escapes and whitespace (ink: \\ keeps the next character; output whitespace collapses)', () => {
	assert.equal(run('\\{x\\} \\| \\\\'), '{x} | \\');
	assert.equal(run('{yes: a\\|b|c}'), 'a|b');
	assert.equal(run('  {yes: a}   {no: b}  c  '), 'a c');
	assert.equal(run('plain  text  stays'), 'plain  text  stays', 'no constructs: as it is');
	assert.equal(run('line 1\n  {yes: two} \nline 3'), 'line 1\ntwo\nline 3');
});

test('AST: spans on every node', () => {
	const text = 'Hi {coins > 1: {a|b}|x} {clock(time)}';
	const { nodes } = parseText(text);
	const spans: string[] = [];
	const visit = (n: TextNode) => {
		spans.push(`${n.type}:${text.slice(n.start, n.end)}`);
		if (n.type === 'cond') [...n.then.nodes, ...(n.else?.nodes ?? [])].forEach(visit);
		if (n.type === 'list') n.items.forEach((c) => c.nodes.forEach(visit));
	};
	nodes.forEach(visit);
	assert.deepEqual(spans, ['text:Hi ', 'cond:{coins > 1: {a|b}|x}', 'text: ', 'list:{a|b}', 'text:a', 'text:b', 'text:x', 'text: ', 'print:{clock(time)}']);
	const cond = nodes[1] as TextNode & { type: 'cond' };
	assert.equal(text.slice(cond.cond.start, cond.cond.end), 'coins > 1');
	assert.equal(text.slice(cond.then.start, cond.then.end), ' {a|b}');
	const expr = parseExpression('max(a, 2) + -b').expr!;
	assert.equal(expr.type, 'binary');
	if (expr.type === 'binary') {
		assert.deepEqual([expr.start, expr.end, expr.left.start, expr.left.end, expr.right.start, expr.right.end], [0, 14, 0, 9, 12, 14]);
	}
});

test('analyser: reads, suggestions, arity, types, messages', () => {
	const a = analyseText('{coins} {map.total} {coins > 1: {has("cucumber")}} {read_chapter(n)} {has("cucumber")} {a|b} {&c|d}', { knownNames: ['coins', 'n'] });
	assert.deepEqual(a.errors, []);
	assert.deepEqual(a.reads.names, [['coins'], ['map', 'total'], ['n']]);
	assert.deepEqual(a.reads.functions, [
		{ name: 'has', literalArgs: ['cucumber'] },
		{ name: 'read_chapter', literalArgs: [null] },
	]);
	assert.equal(a.reads.sequences, 2);
	assert.equal(a.constructs, 8);
	const [e] = analyseText('{map.totl}').errors;
	assert.equal(e.message, 'unknown name `map.totl` – did you mean `map.total`?');
	assert.deepEqual([e.start, e.end], [1, 9]);
	assert.equal(analyseText('{clock()}').errors[0].message, '`clock` expects 1 argument');
	assert.equal(analyseText('{min(1)}').errors[0].message, '`min` expects at least 2 arguments');
	assert.equal(analyseText('{x').errors[0].message, 'missing `}`');
	assert.equal(analyseText('{cois}', { knownNames: ['coins'] }).errors[0].message, 'unknown variable `cois` – did you mean `coins`?');
	assert.equal(analyseText('{cois}', { knownNames: ['coins'] }).errors[0].severity, 'warning');
	assert.deepEqual(codes('{cois}'), [], 'without knownNames any bare name is a story variable');
	assert.equal(analyseText('{time:clock}').errors[0].message, '`{time:clock}` is the old form – write `{clock(time)}`');
	assert.equal(exprMessage({ code: 'nope' }), 'nope');
	assert.equal(didYouMean('totl', ['total', 'tool', 'x']), 'total');
	assert.equal(didYouMean('ab', ['abc']), undefined, 'no guesses for short words');
	const cond = analyseExpression('coins >= 3 and inventory.has("key")');
	assert.deepEqual(cond.errors, []);
	assert.deepEqual(cond.reads.functions, [{ name: 'inventory.has', literalArgs: ['key'] }]);
	assert.deepEqual(analyseExpression('coins >=').errors.map((x) => x.code), ['expected-value']);
	assert.deepEqual(analyseExpression('coins 3').errors.map((x) => x.code), ['unexpected']);
	assert.deepEqual(
		analyseText('{a b} {x}').errors.map((x) => [x.code, x.start]),
		[['unexpected', 3]],
		'one bad construct doesn’t stop the rest',
	);
});

test('limits: length, depth, nodes, constructs, text – reported, fast, never hang', () => {
	const t0 = Date.now();
	assert.ok(codes(`{"${'x'.repeat(600)}"}`).includes('too-long'));
	assert.ok(codes(`{${`${'x'.repeat(60)} + `.repeat(10)}a}`).includes('too-long'));
	assert.ok(codes(`{${'('.repeat(40)}1${')'.repeat(40)}}`).includes('too-deep'));
	assert.ok(codes(`{${'-'.repeat(40)}1}`).includes('too-deep'));
	assert.ok(codes(`{${Array.from({ length: 70 }, () => '1').join('+')}}`).includes('too-many-nodes'));
	assert.deepEqual(codes(`{${Array.from({ length: 32 }, () => '1').join('+')}}`), [], '63 nodes are fine');
	assert.ok(codes('{a}'.repeat(51)).includes('too-many-constructs'));
	assert.deepEqual(codes('{a}'.repeat(50)), []);
	assert.ok(codes(`${'{a|'.repeat(40)}x${'}'.repeat(40)}`).includes('too-deep'));
	assert.ok(codes(`{a} ${'x'.repeat(EXPR_LIMITS.text)}`).includes('text-too-long'));
	assert.equal(run('x'.repeat(20000)).length, 20000, 'a long text without braces is fine');
	// hostile inputs: all must come back quickly
	const hostile = [
		'{'.repeat(100000),
		'}'.repeat(100000),
		'{|'.repeat(50000),
		`{${'"'.repeat(100000)}}`,
		`{${'('.repeat(100000)}`,
		`{${'!'.repeat(100000)}x}`,
		`{a${'|'.repeat(100000)}}`,
		`{${'a.'.repeat(50000)}b}`,
		'\\'.repeat(100001),
		`{x: ${'{y: '.repeat(1000)}`,
		`{${'not '.repeat(10000)}x}`,
		`{${'1+'.repeat(50000)}1}`,
	];
	for (const text of hostile) {
		const start = Date.now();
		const a = analyseText(text);
		evaluateText(text, resolver());
		assert.ok(Date.now() - start < 1000, `slow: ${text.slice(0, 20)}… ${Date.now() - start} ms`);
		assert.ok(Array.isArray(a.errors));
	}
	assert.ok(Date.now() - t0 < 5000);
});

test('fuzz: random texts never throw', () => {
	const alphabet = ['{', '}', '|', '\\', ':', '"', '(', ')', ',', '&', '~', '!', '$', ' ', 'a', 'b', '1', '.', '+', '-', '*', '/', '=', '<', '>', 'and', 'or', 'not', 'mod', 'clock(', 'map.', '\n', '🙂'];
	let seed = 42;
	const rnd = () => ((seed = (seed * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff);
	for (let i = 0; i < 3000; i++) {
		const text = Array.from({ length: Math.floor(rnd() * 30) }, () => alphabet[Math.floor(rnd() * alphabet.length)]).join('');
		const a = analyseText(text, { knownNames: ['a'] });
		const r = evaluateText(text, resolver({ visit: () => i % 4 }));
		assert.equal(typeof r.text, 'string');
		for (const e of a.errors) assert.ok(e.start >= 0 && e.end <= text.length && e.start <= e.end, `${JSON.stringify(text)} ${e.code} ${e.start}-${e.end}`);
		assert.ok(!r.text.includes('{') || text.includes('\\{') || text.includes('"'), `raw code shown: ${JSON.stringify(text)} → ${JSON.stringify(r.text)}`);
	}
});

test('registry: complete entries, pure functions implemented, state functions answered by the resolver', () => {
	const names = new Set<string>();
	for (const f of EXPR_FUNCTIONS) {
		assert.ok(!names.has(f.name), `duplicate ${f.name}`);
		names.add(f.name);
		assert.ok(f.description.de && f.description.en && f.example, f.name);
		assert.ok(f.example.includes(`${f.name}(`), `${f.name}: example uses it`);
		assert.equal(Object.hasOwn(EXPR_IMPLS, f.name), !f.state, `${f.name}: implemented iff pure`);
	}
	assert.deepEqual(Object.keys(EXPR_IMPLS).filter((k) => !names.has(k)), []);
	for (const want of ['clock', 'upper', 'lower', 'capitalize', 'has', 'count_of', 'read_chapter', 'resolved', 'count', 'min', 'max', 'round', 'floor', 'random', 'number'])
		assert.ok(names.has(want), want);
	assert.deepEqual(
		EXPR_NAMESPACES.map((ns) => ns.name),
		['reader', 'read', 'inventory', 'avatar', 'stats', 'effects', 'map', 'now'],
	);
	for (const ns of EXPR_NAMESPACES)
		for (const m of ns.members) {
			assert.ok(m.description.de && m.description.en, `${ns.name}.${m.name}`);
			assert.equal(m.kind === 'function', !!m.params?.length, `${ns.name}.${m.name}: functions take arguments`);
		}
});
