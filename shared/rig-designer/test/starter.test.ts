// The new-module starter is the walk module's source (deck/modules/walk/src/lib.rs) – kept equal to the file in the repository.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { STARTER_FILES } from '../src/starter.ts';
import { fileProblems } from '../src/files.ts';

test('the starter equals deck/modules/walk/src/lib.rs and passes the file limits', () => {
	const walk = readFileSync(new URL('../../../deck/modules/walk/src/lib.rs', import.meta.url), 'utf8');
	assert.equal(STARTER_FILES['lib.rs'], walk);
	assert.deepEqual(fileProblems(STARTER_FILES), []);
});
