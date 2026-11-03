// check-core-only's self-tests (#357, v2 #363): a violating file fails, a clean one passes, each of the 23 workarounds of
// BRIDGE-AUDIT.md §5 fails, every v2 rule has a hit and a miss, the scope follows the import graph, the baseline only shrinks.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { bridgeFiles, compare, context, dataFiles, overBudget, purposes, ratchet, scan, shrunk } from './check-core-only.mjs';
import { emptyContext, measure } from './lib/core-only-rules.mjs';

const fixture = (n) => readFileSync(new URL(`./fixtures/core-only/${n}`, import.meta.url), 'utf8');
const root = new URL('..', import.meta.url).pathname.replace(/^\/(\w:)/, '$1');
const ctx = context(root);
const hits = (text, opts) => scan(text, ctx, opts).counts;

test('a violating file: every shape of deck logic is a hit, an escape without a ticket is refused', () => {
	const s = scan(fixture('violating.ts'), ctx);
	assert.deepEqual(s.counts, { 'scene-literal': 1, 'tempo-math': 2, 'midi-bytes': 2, 'core-text': 1, clamp: 1, random: 1, 'kind-branch': 1 });
	assert.equal(s.bad.length, 1, 'core-only-ok without #N');
	const errors = compare({ 'x.ts': s.counts }, {});
	assert.equal(errors.length, 7);
	assert.match(errors.join('\n'), /x\.ts: 1 × scene-literal – new/);
});

test('a clean file: comments, frame rates, masks, the core\'s line and a ticketed escape pass', () => {
	const s = scan(fixture('clean.ts'), ctx);
	assert.deepEqual(s.counts, {});
	assert.deepEqual(s.bad, []);
	assert.deepEqual([...new Set(s.excused.map((e) => `${e.line} ${e.pattern}`))], ['8 midi-bytes']);
});

// BRIDGE-AUDIT.md §5: 19 of these passed v1 unseen; v2 must fail each, with the rule that names its shape
const WORKAROUNDS = {
	'01-toml-template.ts': 'catalog-literal',
	'02-kind-space-colon.ts': 'catalog-literal',
	'03-kind-quoted-key.ts': 'catalog-literal',
	'04-json-scene.ts': 'catalog-literal',
	'05-node-helper.ts': 'catalog-literal',
	'06-clamp-helper.ts': 'clamp-shape',
	'07-ternary-clamp.ts': 'clamp-shape',
	'08-max-min.ts': 'clamp-shape',
	'09-variable-bounds.ts': 'clamp-shape',
	'10-decimal-status.ts': 'midi-shape',
	'11-crypto-random.ts': 'random-shape',
	'12-lcg.ts': 'random-shape',
	'13-text-from-parts.ts': 'text-shape',
	'14-text-lower-case.ts': 'text-shape',
	'15-switch-kind.ts': 'catalog-branch',
	'16-includes-kind.ts': 'catalog-branch',
	'17-map-lookup.ts': 'catalog-literal',
	'18-mood-thresholds.ts': 'mood-literal',
	'19-renamed-lead.ts': 'lead-shape',
	'20-tempo-renamed.ts': 'tempo-math',
	'21-sixty-over-bpm.ts': 'tempo-math',
	'22-ms-per-beat.ts': 'tempo-math',
	'23-named-midi-const.ts': 'midi-bytes'
};

test('each of the 23 workarounds fails the check', () => {
	const files = readdirSync(new URL('./fixtures/core-only/workarounds/', import.meta.url)).sort();
	assert.deepEqual(files, Object.keys(WORKAROUNDS), 'one fixture per workaround');
	for (const [f, rule] of Object.entries(WORKAROUNDS)) {
		const counts = hits(fixture(`workarounds/${f}`));
		assert.ok(counts[rule] > 0, `${f}: expected a ${rule} hit, got ${JSON.stringify(counts)}`);
		assert.ok(compare({ [f]: counts }, {}).length > 0, `${f} fails against an empty baseline`);
	}
});

test('catalog rules: a catalog kind is a hit, a protocol or wire kind is not', () => {
	assert.equal(hits("export const n = { id: 'a', kind: 'walk' };")['catalog-literal'], undefined, 'v1 counts it (scene-literal), v2 not twice');
	assert.deepEqual(hits("export const n = { id: 'a', kind : 'walk' };"), { 'catalog-literal': 1 });
	assert.deepEqual(hits("export const e = { kind : 'hello', p: {} };"), {}, 'a relay envelope');
	assert.deepEqual(hits("export const f = (e: { kind: string }) => e.kind == 'pong';"), { 'kind-branch': 1 }, 'v1 counts any kind; v2 adds nothing (no catalog kind)');
	assert.deepEqual(hits("export const f = (e: { type: string }) => e.type == 'fft';"), { 'catalog-branch': 1 });
	assert.deepEqual(hits("export const k = ['mixer', 'bus'];"), { 'catalog-literal': 1 }, 'a kind list');
	assert.deepEqual(hits("export const k = ['mixer', 'hello'];"), {});
	assert.deepEqual(hits('export const s = "[[node]]\\nid = \\"x\\"";'), { 'catalog-literal': 1 });
});

test('clamps, thresholds, tempo, MIDI, randomness, lead, tables: a hit and a miss each', () => {
	assert.deepEqual(hits('export const f = (x: number, a: number, b: number) => Math.max(a, Math.min(b, x));'), { 'clamp-shape': 1 });
	assert.deepEqual(hits('export const f = (x: number, y: number) => Math.max(x, y) + Math.min(x, y);'), {});
	assert.deepEqual(hits("export const z = (x: number) => (x > 16 ? 'bad' : x > 10 ? 'warn' : '');"), { threshold: 1 });
	assert.deepEqual(hits('export const f = (t: { tempo: number }, s: number) => s * t.tempo;'), { 'tempo-shape': 1 });
	assert.deepEqual(hits('export const f = (fps: number) => 1000 / fps;'), {});
	assert.deepEqual(hits('export const NOTE_ON = 144;'), { 'midi-shape': 1 });
	assert.deepEqual(hits('export const f = (status: number) => status === 176;'), { 'midi-shape': 1 });
	assert.deepEqual(hits('export const f = (width: number) => width === 176;'), {});
	assert.deepEqual(hits('export const f = (x: number) => { x ^= x << 13; return x; };'), { 'random-shape': 1 });
	assert.deepEqual(hits('export const f = (x: number) => x ^ 13;'), {});
	assert.deepEqual(hits('export let lead = 0; export const g = () => (lead = 3072);'), { 'lead-shape': 1 });
	assert.deepEqual(hits('export let width = 0; export const g = () => (width = 3072);'), {});
	assert.deepEqual(hits('export const S = { min: 0, max: 8, value: 2 };'), { 'spec-table': 1 });
	assert.deepEqual(hits('export const S = { min: 0, max: 8 };'), {});
	assert.deepEqual(hits("export const m = 'laughing';"), { 'mood-literal': 1 });
});

test('texts the core builds: in parts, lower case, a template – the core\'s own line is no hit', () => {
	assert.deepEqual(hits("export const t = 'NO ' + 'SHOW CLOCK';"), { 'text-shape': 1 });
	assert.deepEqual(hits('export const t = (n: number) => `FPS ${n}`;'), { 'text-shape': 1 });
	assert.deepEqual(hits('export const t = (line: string) => line;'), {});
	assert.deepEqual(hits("export const t = 'Show the clock';"), {});
});

test('twins: a Rust array or constant cloned in TS, a comment naming a twin without a ticket', () => {
	const c = emptyContext();
	c.consts.set(0.88, ['deck/x.rs ZOOM']);
	c.arrays.push({ key: '1,2,4,8,10', file: 'deck/x.rs' });
	assert.deepEqual(scan('export const B = [1, 2, 4, 8, 10];\nexport const Z = 0.88;', c).counts, { 'twin-const': 2 });
	assert.deepEqual(scan('export const B = [1, 2, 3, 4];\nexport const Z = 0.5;', c).counts, {});
	assert.deepEqual(scan('// the backoff, as the native link.rs\nexport const x = 1;', c).counts, { 'twin-comment': 1 });
	assert.deepEqual(scan('// the backoff, as the native link.rs (#366 moves it)\nexport const x = 1;', c).counts, {});
});

test('Svelte: only the script blocks are parsed, at their lines', () => {
	const s = scan('<p>{x}</p>\n<script lang="ts">\n\tconst n = { kind : \'fm\' };\n</script>\n<p>kind : \'fm\'</p>', ctx, { svelte: true });
	assert.deepEqual(s.hits.map((h) => [h.rule, h.line]), [['catalog-literal', 3]]);
	assert.deepEqual(measure('<script>\nconst f = () => Math.min(1, 2);\n</script>', { svelte: true }), { lines: 3, functions: 1, math: 1 });
});

test('the scope: importers of a bridge package and of a bridge .ts module, not a component\'s importers', () => {
	const at = mkdtempSync(join(tmpdir(), 'core-only-'));
	try {
		const put = (p, t) => {
			mkdirSync(join(at, p, '..'), { recursive: true });
			writeFileSync(join(at, p), t);
		};
		put('viewer/src/lib/a.ts', "import { Deck } from '@scutoons/live-deck';\nexport const a = Deck;");
		put('viewer/src/lib/b.ts', "import { a } from './a';\nexport const b = a;");
		put('viewer/src/lib/c.svelte', "<script>import { b } from '$lib/b.ts';</script>");
		put('viewer/src/routes/d.svelte', "<script>import C from '$lib/c.svelte';</script>");
		put('viewer/src/lib/e.ts', 'export const e = 1;');
		put('viewer/src/lib/a.test.ts', "import '@scutoons/live-deck';");
		assert.deepEqual(bridgeFiles(at), ['viewer/src/lib/a.ts', 'viewer/src/lib/b.ts', 'viewer/src/lib/c.svelte']);
		put('viewer/static/x.toml', '[[node]]\nid = "o"\nkind = "osc"\n');
		assert.match(dataFiles(at).join('\n'), /viewer\/static\/x\.toml: a scene/);
		put('viewer/static/x.toml', '# GENERATED from deck/native/scenes/x.toml\n[[node]]\n');
		assert.deepEqual(dataFiles(at), []);
	} finally {
		rmSync(at, { recursive: true, force: true });
	}
});

test('budgets and purposes: growth over 5 %, a new escape, a fat new file and a file without a purpose fail', () => {
	const budgets = { 'a.ts': { lines: 100, functions: 10, math: 4, escapes: 0 } };
	assert.deepEqual(overBudget({ 'a.ts': { lines: 105, functions: 10, math: 4, escapes: 0 } }, budgets), []);
	assert.match(overBudget({ 'a.ts': { lines: 106, functions: 10, math: 4, escapes: 0 } }, budgets)[0], /106 lines, its budget is 100/);
	assert.match(overBudget({ 'a.ts': { lines: 10, functions: 1, math: 0, escapes: 1 } }, budgets)[0], /a new escape needs the owner/);
	assert.match(overBudget({ 'new.ts': { lines: 400, functions: 2, math: 0, escapes: 0 } }, budgets)[0], /a new bridge file/);
	assert.deepEqual(purposes(['a.ts'], { 'a.ts': 'adapter: Web MIDI bytes in and out' }), []);
	assert.match(purposes(['a.ts'], {})[0], /without a line in scripts\/bridge-files\.json/);
	assert.match(purposes(['a.ts'], { 'a.ts': 'helps the deck' })[0], /starts with adapter:/);
});

test('the baseline: more hits than it allows fail; it only shrinks against main, unless the owner approved', () => {
	const baseline = { 'a.ts': { clamp: { count: 2, ticket: '#357' } } };
	assert.deepEqual(compare({ 'a.ts': { clamp: 2 } }, baseline), []);
	assert.match(compare({ 'a.ts': { clamp: 3 } }, baseline)[0], /3 × clamp, the baseline allows 2 \(#357\)/);
	// the file shrank: the baseline can follow (never up)
	assert.deepEqual(shrunk({ 'a.ts': { clamp: 1 } }, baseline), { 'a.ts': { clamp: { count: 1, ticket: '#357' } } });
	assert.deepEqual(shrunk({}, baseline), {});
	// an agent grows the baseline to get past: refused, unless a commit carries the owner's approval
	const grown = { 'a.ts': { clamp: { count: 3, ticket: '#357' } }, 'b.ts': { random: { count: 1, ticket: '#999' } } };
	const errors = ratchet({ version: 2, entries: baseline }, { entries: grown }, false);
	assert.equal(errors.length, 2);
	assert.match(errors.join('\n'), /a\.ts clamp grew 2 → 3/);
	assert.match(errors.join('\n'), /b\.ts random is new/);
	assert.deepEqual(ratchet({ version: 2, entries: baseline }, { entries: grown }, true), []);
	assert.deepEqual(ratchet({ version: 2, entries: baseline }, { entries: { 'a.ts': { clamp: { count: 1, ticket: '#357' } } } }, false), []);
});

test('the ratchet v2: allow rules, budgets and known rules only shrink; a v1 baseline knows only its rules and paths', () => {
	const main = {
		version: 2,
		rules: ['clamp', 'spec-table'],
		entries: {},
		allow: [{ file: 'a.ts', rule: 'clamp', match: 'x', count: 1 }],
		budgets: { 'a.ts': { lines: 10, functions: 1, math: 1, escapes: 0 } }
	};
	const now = structuredClone(main);
	now.allow.push({ file: 'a.ts', rule: 'clamp', match: 'y', count: 1 });
	now.allow[0].count = 2;
	now.budgets['a.ts'].lines = 11;
	now.budgets['b.ts'] = { lines: 1, functions: 0, math: 0, escapes: 0 };
	now.entries = { 'a.ts': { 'spec-table': { count: 1, ticket: '#360' }, 'mood-literal': { count: 1, ticket: '#367' } } };
	const e = ratchet(main, now, false).join('\n');
	assert.match(e, /a new allow rule a\.ts clamp "y"/);
	assert.match(e, /allow rule a\.ts clamp "x" grew/);
	assert.match(e, /a\.ts lines budget grew 10 → 11/);
	assert.match(e, /a budget for b\.ts is new/);
	assert.match(e, /a\.ts spec-table is new/);
	assert.doesNotMatch(e, /mood-literal/, 'a rule main did not know starts from today');
	// main's v1 baseline: v2 rules and the import graph's new files start from today's counts
	const v1 = { 'shared/live-deck/src/x.ts': { clamp: { count: 1, ticket: '#357' } } };
	const v2 = { entries: { 'shared/live-deck/src/x.ts': { clamp: { count: 1, ticket: '#369' }, 'spec-table': { count: 3, ticket: '#360' } }, 'viewer/src/lib/live-vote-link.ts': { 'kind-branch': { count: 8, ticket: '#366' } } } };
	assert.deepEqual(ratchet(v1, v2, false), []);
	v2.entries['shared/live-deck/src/x.ts'].clamp.count = 2;
	assert.match(ratchet(v1, v2, false).join('\n'), /x\.ts clamp grew 1 → 2/);
});
