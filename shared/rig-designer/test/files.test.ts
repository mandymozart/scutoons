// `npm test -w @scutoons/rig-designer`: a module's files – the server's rules (ModuleSource) as the designer applies them
// before a request is sent.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { LIMITS, addFile, byteLength, fileOrder, fileProblems, freshName, isDirty, neighbour, normalizeName, removeFile, renameFile, setText, totalBytes } from '../src/files.ts';

const base = () => ({ 'lib.rs': 'mod osc;', 'osc.rs': 'pub fn f() {}' });

test('lib.rs comes first, the others by name', () => {
	assert.deepEqual(fileOrder({ 'z.rs': '', 'a.rs': '', 'lib.rs': '', 'm.rs': '' }), ['lib.rs', 'a.rs', 'm.rs', 'z.rs']);
});

test('a typed name gets its .rs; names follow [a-z0-9_]+.rs', () => {
	assert.equal(normalizeName(' Osc '), 'osc.rs');
	assert.equal(normalizeName('osc.rs'), 'osc.rs');
	assert.equal(normalizeName(''), '');
	const r = addFile(base(), 'filter_2');
	assert.ok(r.ok && r.name === 'filter_2.rs' && r.files['filter_2.rs'] === '');
	for (const bad of ['', 'a b', 'a-b', 'a.txt', 'ä', 'a/b', '.rs']) assert.deepEqual(addFile(base(), bad), { ok: false, problem: 'bad_name' }, bad);
});

test('a file that exists is refused, and so is the 33rd', () => {
	assert.deepEqual(addFile(base(), 'osc'), { ok: false, problem: 'exists' });
	const many: Record<string, string> = { 'lib.rs': '' };
	for (let i = 1; i < LIMITS.maxFiles; i++) many[`f${i}.rs`] = '';
	assert.equal(Object.keys(many).length, 32);
	assert.deepEqual(addFile(many, 'one_more'), { ok: false, problem: 'too_many' });
});

test('rename keeps the text and the order of the others; lib.rs is fixed', () => {
	const r = renameFile(base(), 'osc.rs', 'voice');
	assert.ok(r.ok);
	if (r.ok) {
		assert.deepEqual(Object.keys(r.files), ['lib.rs', 'voice.rs']);
		assert.equal(r.files['voice.rs'], 'pub fn f() {}');
	}
	assert.deepEqual(renameFile(base(), 'lib.rs', 'main'), { ok: false, problem: 'main_fixed' });
	assert.deepEqual(renameFile(base(), 'osc.rs', 'lib'), { ok: false, problem: 'main_fixed' });
	assert.deepEqual(renameFile({ ...base(), 'b.rs': '' }, 'osc.rs', 'b'), { ok: false, problem: 'exists' });
	assert.deepEqual(renameFile(base(), 'osc.rs', 'Bad Name'), { ok: false, problem: 'bad_name' });
	const same = renameFile(base(), 'osc.rs', 'osc.rs');
	assert.ok(same.ok);
});

test('remove drops a file, never lib.rs', () => {
	const r = removeFile(base(), 'osc.rs');
	assert.ok(r.ok);
	if (r.ok) assert.deepEqual(Object.keys(r.files), ['lib.rs']);
	assert.deepEqual(removeFile(base(), 'lib.rs'), { ok: false, problem: 'main_fixed' });
	assert.deepEqual(removeFile(base(), 'nope.rs'), { ok: false, problem: 'bad_name' });
});

test('the limits become problems before a request is sent', () => {
	assert.deepEqual(fileProblems(base()), []);
	assert.deepEqual(fileProblems({}), [{ code: 'empty' }]);
	assert.deepEqual(fileProblems({ 'osc.rs': '' }), [{ code: 'no_main', file: 'lib.rs' }]);
	const big = 'x'.repeat(LIMITS.maxFileBytes + 1);
	assert.deepEqual(fileProblems({ 'lib.rs': big }), [{ code: 'file_too_large', file: 'lib.rs', limit: LIMITS.maxFileBytes, actual: LIMITS.maxFileBytes + 1 }]);
	assert.deepEqual(fileProblems({ 'lib.rs': 'x'.repeat(LIMITS.maxFileBytes) }), [], 'exactly 256 KB is fine');
	const two = { 'lib.rs': 'x'.repeat(LIMITS.maxFileBytes), 'a.rs': 'x'.repeat(LIMITS.maxFileBytes), 'b.rs': 'x' };
	assert.deepEqual(fileProblems(two), [{ code: 'total_too_large', limit: LIMITS.maxTotalBytes, actual: 2 * LIMITS.maxFileBytes + 1 }]);
	assert.deepEqual(fileProblems({ 'lib.rs': '', 'Bad.rs': '' }), [{ code: 'bad_name', file: 'Bad.rs' }]);
	const many: Record<string, string> = { 'lib.rs': '' };
	for (let i = 1; i <= LIMITS.maxFiles; i++) many[`f${i}.rs`] = '';
	assert.deepEqual(fileProblems(many), [{ code: 'too_many', limit: 32, actual: 33 }]);
});

test('sizes are bytes, not characters (the server counts bytes)', () => {
	assert.equal(byteLength('äö'), 4);
	assert.equal(totalBytes({ 'lib.rs': 'ä', 'a.rs': 'ab' }), 4);
});

test('the dirty flag compares names and texts with what was saved', () => {
	const saved = base();
	assert.equal(isDirty(saved, { ...saved }), false);
	assert.equal(isDirty(saved, setText(saved, 'lib.rs', 'changed')), true);
	assert.equal(isDirty(saved, setText(saved, 'lib.rs', saved['lib.rs'])), false);
	const added = addFile(saved, 'new');
	assert.ok(added.ok && isDirty(saved, added.files));
	const renamed = renameFile(saved, 'osc.rs', 'voice');
	assert.ok(renamed.ok && isDirty(saved, renamed.files));
	assert.equal(setText(saved, 'lib.rs', saved['lib.rs']), saved, 'an unchanged text keeps the same object');
});

test('a free name for "+ file"; the file to show after one went away', () => {
	assert.equal(freshName({ 'lib.rs': '' }), 'module.rs');
	assert.equal(freshName({ 'lib.rs': '', 'module.rs': '' }), 'module_2.rs');
	assert.equal(freshName({ 'lib.rs': '', 'module.rs': '', 'module_2.rs': '' }), 'module_3.rs');
	assert.equal(neighbour(base(), 'osc.rs'), 'osc.rs');
	assert.equal(neighbour(base(), 'gone.rs'), 'lib.rs');
	assert.equal(neighbour({ 'a.rs': '' }, 'gone.rs'), 'a.rs');
});
