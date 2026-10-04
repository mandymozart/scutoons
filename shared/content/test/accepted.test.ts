// Accepted warnings: keyed by place (item ids over indices) and a fingerprint of message + value.   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { acceptedWarningKey, catalogWarningPlace, isAcceptedWarning, withAcceptedWarning } from '../src/index.ts';

const fit = { path: ['groups', 0, 'slices', 1, 'dialog', 0, 'text'], message: 'en text is 133% of de (> 130%) – check the fit' };
const chapter = () => ({ groups: [{ slices: [{ dialog: [] }, { dialog: [{ id: 'd7', text: 'Good morning, sunshine' }] }] }] });

test('accepting a warning lists it in the document, accepting again is a no-op, un-accepting empties the field', () => {
	const data: any = chapter();
	assert.equal(isAcceptedWarning(fit, data), false);
	data.accepted_warnings = withAcceptedWarning(fit, data, true);
	assert.equal(data.accepted_warnings.length, 1);
	assert.match(data.accepted_warnings[0], /^groups\.0\.slices\.1\.dialog\.@d7\.text#[0-9a-z]+$/);
	assert.equal(isAcceptedWarning(fit, data), true);
	assert.equal(withAcceptedWarning(fit, data, true)?.length, 1);
	assert.equal(withAcceptedWarning(fit, data, false), undefined);
});

test('a changed value brings the warning back', () => {
	const data: any = chapter();
	data.accepted_warnings = withAcceptedWarning(fit, data, true);
	data.groups[0].slices[1].dialog[0].text = 'Good morning, my sunshine';
	assert.equal(isAcceptedWarning(fit, data), false);
});

test('an item id keeps the key when items move; an index does not', () => {
	const a: any = chapter();
	const moved = { ...fit, path: ['groups', 0, 'slices', 1, 'dialog', 1, 'text'] };
	const b: any = chapter();
	b.groups[0].slices[1].dialog.unshift({ id: 'd1', text: 'Hi' });
	assert.equal(acceptedWarningKey(fit, a), acceptedWarningKey(moved, b));
});

test('a warning about the whole document is keyed by its message', () => {
	const w = { path: [], message: 'no toon section' };
	assert.equal(acceptedWarningKey(w, { a: 1 }), acceptedWarningKey(w, { a: 2 }));
});

test('a catalog warning is stored in its entry (path inside it), a list-wide one in meta/items', () => {
	const catalog = { items: [{ id: 'cam' }, { id: 'vinyl', album: 'awakening', format: 'vinyl', category: 'object' }], albums: [{ id: 'awakening', tracks: [] }] };
	assert.deepEqual(catalogWarningPlace({ path: ['items', 1, 'category'] }, catalog), { kind: 'items', id: 'vinyl', path: ['category'] });
	assert.deepEqual(catalogWarningPlace({ path: ['albums', 0, 'tracks'] }, catalog), { kind: 'albums', id: 'awakening', path: ['tracks'] });
	assert.deepEqual(catalogWarningPlace({ path: ['items'] }, catalog), { kind: 'meta', id: 'items', path: [] });
	assert.equal(catalogWarningPlace({ path: ['stats', 'testo'] }, catalog), null);
	assert.equal(catalogWarningPlace({ path: ['items', 7] }, catalog), null);
});

test('an accepted catalog warning survives a reorder of the list and comes back when the entry changes', () => {
	const warning = { path: ['items', 1, 'category'], message: 'vinyl: an item with an album should be category "media"' };
	const catalog: any = { items: [{ id: 'cam' }, { id: 'vinyl', album: 'awakening', format: 'vinyl', category: 'object' }] };
	const place = catalogWarningPlace(warning, catalog)!;
	const entry = catalog.items[1];
	entry.accepted_warnings = withAcceptedWarning({ path: place.path, message: warning.message }, entry, true);
	// the list moved: the warning now names index 0, the place is still the entry `vinyl`
	catalog.items.reverse();
	const moved = catalogWarningPlace({ path: ['items', 0, 'category'] }, catalog)!;
	assert.equal(moved.id, 'vinyl');
	assert.equal(isAcceptedWarning({ path: moved.path, message: warning.message }, catalog.items[0]), true);
	catalog.items[0].category = 'tool';
	assert.equal(isAcceptedWarning({ path: moved.path, message: warning.message }, catalog.items[0]), false);
});

test('a list-wide catalog warning is keyed by its message', () => {
	const warning = { path: [], message: 'no player plays "tape" (an item with plays: ["tape"])' };
	const meta: any = { version: 1 };
	meta.accepted_warnings = withAcceptedWarning(warning, meta, true);
	assert.match(meta.accepted_warnings[0], /^#[0-9a-z]+$/);
	assert.equal(isAcceptedWarning(warning, { version: 2, accepted_warnings: meta.accepted_warnings }), true);
	assert.equal(isAcceptedWarning({ ...warning, message: 'no player plays "cd" (an item with plays: ["cd"])' }, meta), false);
});
