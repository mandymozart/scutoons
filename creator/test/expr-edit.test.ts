import { test } from 'node:test';
import assert from 'node:assert/strict';
import { evaluateText } from '@scutoons/content';
import { chipSummary, codeOf, fieldOffset, growSelection, modelOf, snapCaret, textParts, textView, expressionView } from '../src/lib/editor/expr-edit.ts';
import { sampleResolver } from '../src/lib/editor/sample-readers.ts';

test('textView: top-level {…} are chips, open ones are not; problems carry their level', () => {
	const text = 'Hi {coins} and {map.totl} {c == 1: coin|coins} {open';
	const { chips, marks } = textView(text, { knownNames: ['coins', 'c'] });
	assert.deepEqual(
		chips.map((c) => [text.slice(c.start, c.end), c.kind]),
		[
			['{coins}', 'print'],
			['{map.totl}', 'print'],
			['{c == 1: coin|coins}', 'cond']
		]
	);
	const unknown = marks.find((x) => x.code === 'unknown-name');
	assert.equal(unknown?.level, 'warning', 'a doubt warns (as validation)');
	assert.equal(unknown?.suggestion, 'map.total');
	assert.equal(text.slice(unknown!.start, unknown!.end), 'map.totl');
	assert.equal(marks.find((x) => x.code === 'missing-close')?.level, 'error');
	// a closed construct that doesn't parse is a chip with its error
	const bad = textView('a {coins ==} b');
	assert.deepEqual(bad.chips.map((c) => c.kind), ['bad']);
	assert.equal(bad.marks[0].level, 'error');
});

test('textView: a text box’s lines are analysed one by one, offsets in the joined field', () => {
	const text = 'one {a}\n\ntwo {b';
	assert.deepEqual(textParts(text, true), [
		{ start: 0, text: 'one {a}' },
		{ start: 9, text: 'two {b' }
	]);
	const { chips, marks } = textView(text, { paragraphs: true });
	assert.equal(chips.length, 1);
	assert.equal(marks[0].start, 13, 'the open brace of line 2');
	assert.equal(fieldOffset(text, 1, 4, true), 13);
	assert.equal(fieldOffset(text, undefined, 4, true), 4);
});

test('expressionView: a condition’s problems', () => {
	assert.equal(expressionView('coins >=').marks[0].level, 'error');
	assert.equal(expressionView('coins >= 3', ['coins']).marks.length, 0);
	assert.equal(expressionView('coinz >= 3', ['coins']).marks[0].level, 'warning');
});

test('caret: steps over chips, selections grow to whole chips', () => {
	const { chips } = textView('ab {coins} cd');
	assert.equal(snapCaret(chips, 5, 3), 10, 'from the left → after the chip');
	assert.equal(snapCaret(chips, 8, 10), 3, 'from the right → before it');
	assert.equal(snapCaret(chips, 1, 0), 1, 'outside: unchanged');
	assert.deepEqual(growSelection(chips, 5, 12), [3, 12]);
});

test('builder: chips read back into the model and written again', () => {
	const value = modelOf('{coins}');
	assert.equal(value.kind, 'value');
	assert.equal(value.left, 'coins');
	assert.equal(codeOf(value), '{coins}');

	const cmp = modelOf('{map.seen >= map.total - 1}');
	assert.deepEqual([cmp.kind, cmp.left, cmp.op, cmp.right], ['value', 'map.seen', '>=', 'map.total - 1']);
	assert.equal(modelOf('{c:coin}').then, 'coin');

	const cond = modelOf('{coins == 1: coin|coins}');
	assert.deepEqual([cond.kind, cond.left, cond.op, cond.right, cond.then, cond.else], ['cond', 'coins', '==', '1', 'coin', 'coins']);
	assert.equal(codeOf(cond), '{coins == 1: coin|coins}');
	assert.equal(codeOf({ ...cond, else: '' }), '{coins == 1: coin}');

	const list = modelOf('{&Hi|Hello|Hey}');
	assert.deepEqual([list.kind, list.list, list.items], ['list', 'cycle', ['Hi', 'Hello', 'Hey']]);
	assert.equal(codeOf(list), '{&Hi|Hello|Hey}');
	assert.equal(codeOf({ ...list, list: 'sequence' }), '{Hi|Hello|Hey}');
	assert.equal(codeOf({ ...list, list: 'sequence', items: ['only'] }), '{$only}');
	assert.equal(modelOf('{$a|b}').list, 'sequence');

	assert.equal(modelOf('{~!a|b}').kind, 'code', 'shuffle once stays code');
	assert.equal(modelOf('{coins ==}').kind, 'code', 'a syntax error opens as code');
	assert.equal(modelOf('{a} {b}').kind, 'code');

	const expr = modelOf('count("e1") < 3', true);
	assert.deepEqual([expr.kind, expr.left, expr.op, expr.right], ['value', 'count("e1")', '<', '3']);
	assert.equal(codeOf(expr, true), 'count("e1") < 3');
	assert.equal(modelOf('a and b', true).left, 'a and b');
});

test('chipSummary: readable lines', () => {
	assert.equal(chipSummary('{coins == 1: coin|coins}'), 'coins == 1 ? coin | coins');
	assert.equal(chipSummary('{clock(t)}'), 'clock(t)');
	assert.equal(chipSummary('{&a|b}'), 'cycle: a | b');
});

test('sample readers: a guest, halfway, everything collected', () => {
	const text = '{reader.name: Hi {reader.name}|Hi guest} {map.seen}/{map.total} {has("coin"): rich|poor} {a|b|c} {coins}';
	assert.equal(evaluateText(text, sampleResolver('guest', 'en')).text, 'Hi guest 1/12 poor a');
	assert.equal(evaluateText(text, sampleResolver('halfway', 'en')).text, 'Hi Mia 8/12 rich b 3');
	assert.equal(evaluateText(text, sampleResolver('complete', 'en')).text, 'Hi Mia 12/12 rich c 10');
	assert.equal(evaluateText('{reader.language}', sampleResolver('guest', 'de')).text, 'de');
});
