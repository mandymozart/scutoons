// Items are not variables (agents/BACKLOG.md 10.38): `inventory.<item>` reads how many of a catalog item the reader has –
// checked against the catalog; a namespace is never a variable's domain.
//   node --experimental-transform-types --no-warnings --test test/inventory-names.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { analyseExpression, analyseText, isStoryPath, isWrittenVariable, validateFile } from '../src/index.ts';

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

const chapter = (events: object[]) => ({ number: 1, groups: [{ label: 'a', slices: [{ image: 'x.webp' }], events }] });
const check = (events: object[], items?: string[]) =>
	validateFile('chapter', chapter(events), 'de', { file: 'c', ...(items ? { items } : {}) } as never).filter((p) => /item|give|qty/.test(p.message));

test('giveItem / a choice\'s give: a catalog item, 1 … 10 pieces', () => {
	const items = ['cucumber'];
	assert.deepEqual(check([{ type: 'giveItem', params: { item: 'cucumber', qty: 2 } }], items), []);
	assert.match(check([{ type: 'giveItem', params: { item: '' } }], items)[0].message, /giveItem needs an item/);
	assert.match(check([{ type: 'giveItem', params: { item: 'cucumbr' } }], items)[0].message, /not in the item catalog/);
	assert.match(check([{ type: 'giveItem', params: { item: 'cucumber', qty: 11 } }], items)[0].message, /qty is a whole number 1 … 10/);
	const choice = (onSelect: object) => ({ type: 'addChoice', params: { label: { de: 'Nimm' }, onSelect } });
	assert.deepEqual(check([choice({ give: { item: 'cucumber' } })], items), []);
	assert.equal(check([choice({ give: { item: 'cucumber' } })], items).length, 0);
	assert.ok(!validateFile('chapter', chapter([choice({ give: { item: 'cucumber' } })]), 'de', { file: 'c', items } as never).some((p) => p.message.includes('an action is')));
});

test('a story variable named like a catalog item warns (write: giveItem; read: inventory.<item>)', () => {
	const items = ['cucumber'];
	const set = check([{ type: 'setVariable', params: { variable: 'cucumber', value: 1, op: 'add' } }], items);
	assert.equal(set.length, 1);
	assert.equal(set[0].level, 'warning');
	assert.match(set[0].message, /catalog item, not a story variable – give it with giveItem/);
	const add = check([{ type: 'addChoice', params: { label: { de: 'x' }, onSelect: { add: { variable: 'global.cucumber', value: 1 } } } }], items);
	assert.match(add[0].message, /"global.cucumber" is a catalog item/);
	const read = check([{ type: 'showMessage', params: { text: { de: 'x' } }, if: { variable: 'cucumber', op: '>', value: 0 } }], items);
	assert.match(read[0].message, /what the reader has is inventory.cucumber/);
	assert.deepEqual(check([{ type: 'setVariable', params: { variable: 'cucumber', value: 1 } }]), [], 'without the catalog nothing is judged');
	// in an expression: a bare item name is a story variable – the suggestion is inventory.<item>
	const e = analyseExpression('cucumber > 0', { knownItems: items }).errors[0];
	assert.equal(e.code, 'item-as-variable');
	assert.equal(e.suggestion, 'inventory.cucumber');
});

test('takeItem / a choice\'s take: a catalog item, 1 … 10, a valid repeat rule', () => {
	const items = ['cucumber', 'lsd'];
	assert.deepEqual(check([{ type: 'takeItem', params: { item: 'cucumber', qty: 5, repeat: { mode: 'interval', every: { days: 1 } } } }], items), []);
	assert.match(check([{ type: 'takeItem', params: { item: '' } }], items)[0].message, /takeItem needs an item/);
	assert.match(check([{ type: 'takeItem', params: { item: 'cucumber', qty: 0 } }], items)[0].message, /qty is a whole number/);
	const bad = validateFile('chapter', chapter([{ type: 'takeItem', params: { item: 'cucumber', repeat: { mode: 'sometimes' } } }]), 'de', { file: 'c', items } as never);
	assert.ok(bad.some((p) => p.level === 'error' && /repeat\.mode/.test(p.message)));
	const trade = { type: 'addChoice', params: { label: { de: 'Tauschen' }, onSelect: [{ take: { item: 'cucumber', qty: 5 } }, { give: { item: 'lsd' } }] } };
	assert.ok(!validateFile('chapter', chapter([trade]), 'de', { file: 'c', items } as never).some((p) => p.message.includes('an action is')));
});
