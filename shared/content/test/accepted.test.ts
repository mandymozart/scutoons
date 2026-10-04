// Accepted warnings: keyed by place (item ids over indices) and a fingerprint of message + value.   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { acceptedWarningKey, isAcceptedWarning, withAcceptedWarning } from '../src/index.ts';

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
