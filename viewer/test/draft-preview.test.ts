// The creator's draft preview: what an address switches (lib/draft-preview-link.ts) and the chapter page's gate
// (lib/chapter-gate.ts) – a draft toon / chapter / a chapter no season lists opens for a creator, never for a reader.
//   npm test -w viewer
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { linkedPreview } from '../src/lib/draft-preview-link.ts';
import { chapterGate } from '../src/lib/chapter-gate.ts';

test('the address switches the preview: ?draft=1 / ?preview=draft on, ?draft=0 off, else nothing', () => {
	assert.equal(linkedPreview('?draft=1'), true);
	assert.equal(linkedPreview('?label=x&draft=1'), true);
	assert.equal(linkedPreview('?preview=draft'), true);
	assert.equal(linkedPreview('?draft=0'), false);
	assert.equal(linkedPreview('?draft=0&preview=draft'), false, 'an explicit draft=0 wins');
	assert.equal(linkedPreview(''), null);
	assert.equal(linkedPreview('?draft=yes'), null);
	assert.equal(linkedPreview('?preview=1'), null);
});

test('a creator in the preview reads draft, coming-soon and unlisted chapters', () => {
	for (const status of ['draft', 'coming-soon', 'published', null]) {
		assert.equal(chapterGate(status, true, true), null, String(status));
	}
});

test('a reader gets the gate for drafts and unlisted chapters, coming soon for coming-soon', () => {
	assert.equal(chapterGate('draft', false, false), 'draft');
	assert.equal(chapterGate(null, false, false), 'draft', 'a chapter no published season lists');
	assert.equal(chapterGate('coming-soon', false, false), 'coming-soon');
	assert.equal(chapterGate('published', false, false), null);
	assert.equal(chapterGate('archived', false, false), 'draft', 'an unknown status is not public');
});

test('a dev build reads drafts without the preview, coming soon still gated', () => {
	assert.equal(chapterGate('draft', false, true), null);
	assert.equal(chapterGate(null, false, true), null);
	assert.equal(chapterGate('coming-soon', false, true), 'coming-soon');
});
