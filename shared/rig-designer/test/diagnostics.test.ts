// `npm test -w @scutoons/rig-designer`: the compiler's messages as markers, a problems list and a jump target.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { counts, jumpTarget, markersFor, offsetAt, placeOf, rangeFor, readDiagnostics, sortProblems, worstByFile } from '../src/diagnostics.ts';
import type { Diagnostic } from '../src/types.ts';

const d = (file: string, line: number, level = 'error', message = 'm', column = 1): Diagnostic => ({ file, line, column, level, message });
const files = { 'lib.rs': 'fn a() {\n    let x = 1;\n}\n', 'osc.rs': 'pub fn f() {}' };

test('readDiagnostics keeps what an answer has and repairs what it lacks', () => {
	assert.deepEqual(readDiagnostics(null), []);
	assert.deepEqual(readDiagnostics('x'), []);
	assert.deepEqual(readDiagnostics([{ file: 'lib.rs', line: 3, column: 5, level: 'warning', message: 'unused' }, 7, null, { message: 'bare' }]), [
		{ file: 'lib.rs', line: 3, column: 5, level: 'warning', message: 'unused' },
		{ file: '', line: 0, column: 0, level: 'error', message: 'bare' }
	]);
	assert.equal(readDiagnostics([{ line: -4 }])[0].line, 0);
});

test('problems: errors first, then by file (lib.rs first), line and column', () => {
	const list = [d('osc.rs', 1, 'warning'), d('osc.rs', 9), d('lib.rs', 12, 'warning'), d('lib.rs', 4, 'error', 'b', 9), d('lib.rs', 4, 'error', 'a', 2), d('gone.rs', 1)];
	assert.deepEqual(
		sortProblems(list, files).map((x) => `${x.level[0]}:${x.file}:${x.line}:${x.column}`),
		['e:lib.rs:4:2', 'e:lib.rs:4:9', 'e:osc.rs:9:1', 'e:gone.rs:1:1', 'w:lib.rs:12:1', 'w:osc.rs:1:1']
	);
	assert.equal(list[0].file, 'osc.rs', 'the input is not reordered');
});

test('counts and the dots on the file tabs', () => {
	const list = [d('lib.rs', 1, 'warning'), d('lib.rs', 2, 'error'), d('osc.rs', 1, 'warning'), d('', 0, 'error')];
	assert.deepEqual(counts(list), { errors: 2, warnings: 2 });
	assert.deepEqual(worstByFile(list), { 'lib.rs': 'error', 'osc.rs': 'warning' });
});

test('markers of one file: only messages that name a line', () => {
	const list = [d('lib.rs', 2, 'error', 'bad', 9), d('lib.rs', 0, 'error', 'general'), d('osc.rs', 1), d('lib.rs', 1, 'note', 'n', 0)];
	assert.deepEqual(markersFor('lib.rs', list), [
		{ line: 2, column: 9, level: 'error', message: 'bad' },
		{ line: 1, column: 1, level: 'info', message: 'n' }
	]);
});

test('a click jumps to file:line:column – nowhere for a message without a place or a file that is gone', () => {
	assert.deepEqual(jumpTarget(d('osc.rs', 3, 'error', 'x', 4), files), { file: 'osc.rs', line: 3, column: 4 });
	assert.deepEqual(jumpTarget(d('osc.rs', 3, 'error', 'x', 0), files), { file: 'osc.rs', line: 3, column: 1 });
	assert.equal(jumpTarget(d('osc.rs', 0), files), null);
	assert.equal(jumpTarget(d('gone.rs', 2), files), null);
	assert.equal(jumpTarget(d('', 2), files), null);
});

test('line / column to a document offset, clamped', () => {
	const text = 'ab\ncde\n\nf';
	assert.equal(offsetAt(text, 1, 1), 0);
	assert.equal(offsetAt(text, 2, 3), 5);
	assert.equal(offsetAt(text, 2, 99), 6, 'a column past the line end sits at its end');
	assert.equal(offsetAt(text, 3, 1), 7, 'an empty line');
	assert.equal(offsetAt(text, 4, 1), 8);
	assert.equal(offsetAt(text, 99, 1), text.length, 'a line past the end is the end');
	assert.equal(offsetAt(text, 0, 0), 0);
});

test('a marker underlines the word at the place, at least one character', () => {
	const text = 'let value = other;\nfn';
	assert.deepEqual(rangeFor(text, 1, 5), { from: 4, to: 9 });
	assert.deepEqual(rangeFor(text, 1, 11), { from: 10, to: 11 }, 'punctuation: one character');
	assert.deepEqual(rangeFor(text, 1, 99), { from: 17, to: 18 }, 'past the line end: the last character');
	assert.deepEqual(rangeFor('', 1, 1), { from: 0, to: 0 });
});

test('the place as text', () => {
	assert.equal(placeOf(d('lib.rs', 12, 'error', 'x', 5)), 'lib.rs:12:5');
	assert.equal(placeOf(d('lib.rs', 12, 'error', 'x', 0)), 'lib.rs:12');
	assert.equal(placeOf(d('lib.rs', 0)), 'lib.rs');
	assert.equal(placeOf(d('', 0)), '');
});
