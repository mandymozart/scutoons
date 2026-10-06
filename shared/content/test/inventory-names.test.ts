// Items are not variables (agents/BACKLOG.md 10.38): `inventory.<item>` reads how many of a catalog item the reader has –
// checked against the catalog; a namespace is never a variable's domain.
//   node --experimental-transform-types --no-warnings --test test/inventory-names.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { analyseExpression, analyseText, isStoryPath, isWrittenVariable } from '../src/index.ts';

const codes = (r: { errors: { code: string; suggestion?: string }[] }) => r.errors.map((e) => `${e.code}${e.suggestion ? `→${e.suggestion}` : ''}`);

test('inventory.<item>: a catalog item is a number, an unknown one warns with a suggestion', () => {
	const knownItems = ['cucumber', 'camera'];
	assert.deepEqual(codes(analyseExpression('inventory.cucumber > 0', { knownItems })), []);
	assert.deepEqual(codes(analyseText('You have {inventory.cucumber} cucumbers.', { knownItems })), []);
	assert.deepEqual(codes(analyseExpression('inventory.cucumbr > 0', { knownItems })), ['unknown-item→inventory.cucumber']);
	assert.equal(analyseExpression('inventory.cucumbr', { knownItems }).errors[0].severity, 'warning');
	// without the catalog: any item is taken
	assert.deepEqual(codes(analyseExpression('inventory.whatever')), []);
	// the namespace's own members still win: a function needs its call, a value is a value
	assert.deepEqual(codes(analyseExpression('inventory.has', { knownItems })), ['function-needs-call']);
	assert.deepEqual(codes(analyseExpression('inventory.items > 2', { knownItems })), []);
	// other namespaces stay closed
	assert.equal(codes(analyseExpression('reader.cucumber'))[0]?.split('→')[0], 'unknown-name');
	// and it is no story variable
	assert.equal(isStoryPath(['inventory', 'cucumber']), false);
});

test('a namespace is never a variable domain: inventory.x / reader.x can not be written', () => {
	assert.equal(isWrittenVariable('cucumber'), true);
	assert.equal(isWrittenVariable('global.cucumber'), true);
	assert.equal(isWrittenVariable('toptoy.cucumber'), true);
	assert.equal(isWrittenVariable('inventory.cucumber'), false);
	assert.equal(isWrittenVariable('reader.name'), false);
});
