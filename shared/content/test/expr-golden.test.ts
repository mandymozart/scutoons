// Expressions in texts (agents/EXPRESSIONS.md §4): the golden fixtures test/fixtures/expr/*.json – text → AST summary,
// problems, reads and the evaluated text with a fixed resolver. The PHP port (server/tests/Unit/Domain/ExprTest.php)
// reads the same files and must give the same AST summary, problems and reads.
// Refresh the expected values after a deliberate change: UPDATE_EXPR_FIXTURES=1 node --experimental-transform-types --test test/expr-golden.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { analyseText, type ExprAnalysis } from '../src/expr/analyse.ts';
import type { Content, Expr, ExprValue, TextNode } from '../src/expr/ast.ts';
import { evaluateText, type ExprResolver } from '../src/expr/evaluate.ts';
import { parseText } from '../src/expr/parse.ts';
import { EXPR_FUNCTIONS, EXPR_NAMESPACES } from '../src/expr/registry.ts';

const DIR = new URL('./fixtures/expr/', import.meta.url);
const UPDATE = !!process.env.UPDATE_EXPR_FIXTURES;

/** the AST as a short S-expression (the PHP test builds the same) */
export function summary(nodes: TextNode[]): string {
	const expr = (e: Expr): string => {
		switch (e.type) {
			case 'num':
			case 'str':
			case 'bool':
				return JSON.stringify(e.value);
			case 'name':
				return e.path.join('.');
			case 'call':
				return `(call ${e.name}${e.args.map((a) => ' ' + expr(a)).join('')})`;
			case 'unary':
				return `(${e.op} ${expr(e.arg)})`;
			case 'binary':
				return `(${e.op} ${expr(e.left)} ${expr(e.right)})`;
		}
	};
	const content = (c: Content | null): string => (c ? `[${c.nodes.map(node).join(' ')}]` : 'nil');
	const node = (n: TextNode): string => {
		switch (n.type) {
			case 'text':
				return JSON.stringify(n.value);
			case 'print':
				return `(print ${expr(n.expr)})`;
			case 'cond':
				return `(if ${expr(n.cond)} ${content(n.then)} ${content(n.else)})`;
			case 'list':
				return `(${n.kind}#${n.index}${n.items.map((c) => ' ' + content(c)).join('')})`;
			case 'bad':
				return 'bad';
		}
	};
	return nodes.map(node).join(' ');
}

interface FixtureResolver {
	names?: Record<string, ExprValue>;
	/** `name(args as JSON without brackets)` → value, e.g. `has("cucumber")` */
	calls?: Record<string, ExprValue>;
	visits?: Record<string, number>;
	seed?: string | number;
	language?: string;
}

interface Case {
	text: string;
	note?: string;
	resolver?: FixtureResolver;
	ast?: string;
	errors?: { code: string; severity: string; start: number; end: number; message: string }[];
	reads?: ExprAnalysis['reads'];
	constructs?: number;
	out?: string;
	runtime?: string[];
	visited?: number[];
}

interface Fixture {
	description: string;
	resolver: FixtureResolver;
	knownNames?: string[];
	cases: Case[];
}

/** a resolver from plain data – own keys only (`__proto__` / `constructor` are no names) */
export function fixtureResolver(r: FixtureResolver): ExprResolver {
	const own = <T>(map: Record<string, T> | undefined, key: string): T | undefined => (map && Object.hasOwn(map, key) ? map[key] : undefined);
	return {
		name: (path) => own(r.names, path.join('.')),
		call: (name, args) => own(r.calls, `${name}(${JSON.stringify(args).slice(1, -1)})`),
		visit: (index) => own(r.visits, String(index)) ?? 0,
		seed: r.seed,
		language: r.language,
	};
}

export function expected(fixture: Fixture, c: Case): Case {
	const parsed = parseText(c.text);
	const analysis = analyseText(c.text, fixture.knownNames ? { knownNames: fixture.knownNames } : {});
	const result = evaluateText(c.text, fixtureResolver({ ...fixture.resolver, ...c.resolver }));
	return {
		text: c.text,
		...(c.note ? { note: c.note } : {}),
		...(c.resolver ? { resolver: c.resolver } : {}),
		ast: summary(parsed.nodes),
		errors: analysis.errors.map((e) => ({ code: e.code, severity: e.severity, start: e.start, end: e.end, message: e.message })),
		reads: analysis.reads,
		constructs: analysis.constructs,
		out: result.text,
		runtime: result.problems.filter((p) => !analysis.errors.some((e) => e.code === p.code && e.start === p.start)).map((p) => `${p.code}@${p.start}`),
		visited: result.visited,
	};
}

/** the registry as the PHP port needs it (no descriptions) */
export function registryExport() {
	return {
		functions: EXPR_FUNCTIONS.map((f) => ({ name: f.name, params: f.params, variadic: !!f.variadic, result: f.result, state: !!f.state })),
		namespaces: EXPR_NAMESPACES.map((ns) => ({ name: ns.name, members: ns.members.map((m) => ({ name: m.name, kind: m.kind, type: m.type, params: m.params ?? [] })) })),
	};
}

const files = readdirSync(DIR).filter((f) => f.endsWith('.json') && f !== 'registry.json');

test('golden fixtures: there are some', () => assert.ok(files.length >= 6, files.join(', ')));

for (const file of files) {
	test(`golden fixtures: ${file}`, () => {
		const url = new URL(file, DIR);
		const fixture = JSON.parse(readFileSync(url, 'utf8')) as Fixture;
		const cases = fixture.cases.map((c) => expected(fixture, c));
		if (UPDATE) {
			writeFileSync(url, JSON.stringify({ ...fixture, cases }, null, '\t') + '\n');
			return;
		}
		cases.forEach((c, i) => assert.deepEqual(fixture.cases[i], c, `${file} #${i}: ${JSON.stringify(c.text)}`));
	});
}

test('golden fixtures: registry.json is the registry', () => {
	const url = new URL('registry.json', DIR);
	if (UPDATE) writeFileSync(url, JSON.stringify(registryExport(), null, '\t') + '\n');
	assert.deepEqual(JSON.parse(readFileSync(url, 'utf8')), registryExport());
});
