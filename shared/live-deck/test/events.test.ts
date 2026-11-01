// `npm test -w shared/live-deck` (#236): the story adapter gives the events deck-events gives (test/fixtures/events.json,
// written and kept current by `cargo test -p deck-events`), and the real chapter's slice event becomes a story.showMessage.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fromStory, MAX_DATA, type DeckEvent } from '../src/events.ts';

interface Row {
	label: string;
	story: Parameters<typeof fromStory>[0];
	source: string | null;
	event: DeckEvent | null;
}
const ROWS = JSON.parse(readFileSync(new URL('./fixtures/events.json', import.meta.url), 'utf8')) as Row[];
const CHAPTER = JSON.parse(readFileSync(new URL('../../content/test/fixtures/neutral/chapter.json', import.meta.url), 'utf8'));

test('every shared case is the same event in TypeScript and Rust', () => {
	assert.ok(ROWS.length >= 7);
	for (const r of ROWS) {
		assert.deepEqual(fromStory(r.story, r.source === null ? {} : { source: r.source }), r.event, r.label);
	}
});

test("the chapter's slice event becomes a story.showMessage trigger carrying catpope's mood", () => {
	const found: StoryLike[] = [];
	(function walk(v: unknown): void {
		if (Array.isArray(v)) v.forEach(walk);
		else if (v && typeof v === 'object') {
			const o = v as StoryLike;
			if (o.type === 'showMessage' && 'trigger' in o) found.push(o);
			Object.values(o).forEach(walk);
		}
	})(CHAPTER);
	assert.ok(found.length > 0, 'the chapter has a slice showMessage');
	const e = fromStory(found[0])!;
	assert.equal(e.name, 'story.showMessage');
	assert.equal(e.start, 'now');
	assert.equal(e.props, undefined);
	assert.equal(e.data?.character, 'catpope');
	assert.equal(e.data?.mood, 'happy');
	assert.equal('trigger' in e, false, 'the trigger was the caller\'s');
});

test('the runner keeps choices, a big payload is refused with a hint, a bad shape throws', () => {
	assert.equal(fromStory({ type: 'showChoices' }), null);
	assert.equal(fromStory({ type: 'jumpTo', params: { target: 'x' } }), null);
	assert.throws(() => fromStory({ type: 'showMessage', params: { text: 'x'.repeat(MAX_DATA) } }), /send a reference/);
	assert.throws(() => fromStory({ type: 'Bad Name' }), /event name/);
	assert.throws(() => fromStory({ type: 'fx', params: 3 as never }), /params/);
});

interface StoryLike {
	type: string;
	params?: Record<string, unknown>;
	trigger?: unknown;
}
