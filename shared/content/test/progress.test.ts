// Unit tests of the start button's reading state (readingState / advanceCheckpoint, agents/DESIGN.md "Start button").
//   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { advanceCheckpoint, chapterCheckpointUri, chapterProgress, nextStep, readingState, type Checkpoint } from '../src/index.ts';

const cp = (n: number, position: number | undefined, isComplete: boolean, t: string, toon = 't'): Checkpoint => ({
	uri: chapterCheckpointUri(toon, n),
	isComplete,
	...(position === undefined ? {} : { position }),
	created_at: t,
	updated_at: t
});

test('readingState: nothing read → the first readable chapter', () => {
	assert.deepEqual(readingState('t', [1, 2], []), { kind: 'start', chapter: 1 });
	assert.deepEqual(readingState('t', [1, 2], undefined), { kind: 'start', chapter: 1 });
	// checkpoints of other toons / unlisted (coming soon) chapters don't count
	assert.deepEqual(readingState('t', [1, 2], [cp(1, 0.5, false, '2026-01-01', 'other'), cp(3, 0.5, false, '2026-01-02')]), { kind: 'start', chapter: 1 });
	assert.equal(readingState('t', [], []), null);
});

test('readingState: the last opened chapter decides', () => {
	// started, not finished → continue it (with its position)
	assert.deepEqual(readingState('t', [1, 2, 3], [cp(1, 1, true, '2026-01-01'), cp(2, 0.4, false, '2026-01-02')]), { kind: 'continue', chapter: 2, position: 0.4 });
	// finished → the next readable chapter
	assert.deepEqual(readingState('t', [1, 2, 3], [cp(1, 1, true, '2026-01-01')]), { kind: 'next', chapter: 2 });
	// finished the last one, an earlier one never finished → that one
	assert.deepEqual(readingState('t', [1, 2], [cp(2, 1, true, '2026-01-02')]), { kind: 'next', chapter: 1 });
	// everything finished → again from the start
	assert.deepEqual(readingState('t', [1, 2], [cp(1, 1, true, '2026-01-01'), cp(2, 1, true, '2026-01-02')]), { kind: 'again', chapter: 1 });
	// re-reading chapter 1 after finishing everything → continue chapter 1
	assert.deepEqual(readingState('t', [1, 2], [cp(1, 0.2, true, '2026-01-03'), cp(2, 1, true, '2026-01-02')]), { kind: 'continue', chapter: 1, position: 0.2 });
	// older checkpoints without position
	assert.deepEqual(readingState('t', [1, 2], [cp(1, undefined, false, '2026-01-01')]), { kind: 'continue', chapter: 1, position: 0 });
	assert.deepEqual(readingState('t', [1, 2], [cp(1, undefined, true, '2026-01-01')]), { kind: 'next', chapter: 2 });
});

test('readingState: the prologue (chapter 0) first, then the chapters as before', () => {
	// fresh visitor → the prologue
	assert.deepEqual(readingState('t', [0, 1, 2], []), { kind: 'prologue', chapter: 0 });
	// watched part of it → still the prologue
	assert.deepEqual(readingState('t', [0, 1, 2], [cp(0, 0.4, false, '2026-01-01')]), { kind: 'prologue', chapter: 0 });
	// watched to its end (ended / ≥ 95 % = position 1) → chapter 1
	assert.deepEqual(readingState('t', [0, 1, 2], [cp(0, 1, true, '2026-01-01')]), { kind: 'start', chapter: 1 });
	// a reader of chapter 1 who never watched it → the chapters' rules
	assert.deepEqual(readingState('t', [0, 1, 2], [cp(1, 0.3, false, '2026-01-01')]), { kind: 'continue', chapter: 1, position: 0.3 });
	assert.deepEqual(readingState('t', [0, 1, 2], [cp(1, 1, true, '2026-01-01')]), { kind: 'next', chapter: 2 });
	// the prologue watched again after chapter 1 → chapter 1 is still the last opened chapter
	assert.deepEqual(readingState('t', [0, 1, 2], [cp(1, 0.3, false, '2026-01-01'), cp(0, 1, true, '2026-01-02')]), { kind: 'continue', chapter: 1, position: 0.3 });
	// everything finished → "again" from chapter 1, never the prologue
	assert.deepEqual(readingState('t', [0, 1, 2], [cp(0, 1, true, '2026-01-01'), cp(1, 1, true, '2026-01-02'), cp(2, 1, true, '2026-01-03')]), { kind: 'again', chapter: 1 });
	assert.deepEqual(readingState('t', [0, 1], [cp(1, 1, true, '2026-01-02')]), { kind: 'again', chapter: 1 });
	// only a prologue is readable
	assert.deepEqual(readingState('t', [0], [cp(0, 1, true, '2026-01-01')]), { kind: 'prologue', chapter: 0 });
	// chapter 0 in the list progress like any chapter
	assert.deepEqual(chapterProgress('t', 0, [cp(0, 0.5, false, '2026-01-01')]), { position: 0.5, finished: false });
});

test('advanceCheckpoint: positions only grow, a finished chapter opened again starts over', () => {
	const uri = chapterCheckpointUri('t', 1);
	const first = advanceCheckpoint(undefined, uri, 0.1, true, 'a')!;
	assert.deepEqual([first.position, first.isComplete, first.updated_at], [0.1, false, 'a']);
	assert.equal(advanceCheckpoint(first, uri, 0.05, false, 'b'), null, 'going back changes nothing');
	const later = advanceCheckpoint(first, uri, 0.5, false, 'b')!;
	assert.deepEqual([later.position, later.isComplete], [0.5, false]);
	const reopened = advanceCheckpoint(later, uri, 0.02, true, 'c')!;
	assert.deepEqual([reopened.position, reopened.updated_at], [0.5, 'c'], 'a new visit keeps the position, touches the time');
	const done = advanceCheckpoint(reopened, uri, 1, false, 'd')!;
	assert.deepEqual([done.position, done.isComplete], [1, true]);
	const again = advanceCheckpoint(done, uri, 0.02, true, 'e')!;
	assert.deepEqual([again.position, again.isComplete, again.created_at], [0.02, true, 'a']);
});

test('chapterProgress: never opened, in progress, finished and read again', () => {
	const uri = chapterCheckpointUri('t', 2);
	assert.equal(chapterProgress('t', 2, []), null);
	assert.deepEqual(chapterProgress('t', 2, [{ uri, isComplete: false, position: 0.4 } as Checkpoint]), { position: 0.4, finished: false });
	assert.deepEqual(chapterProgress('t', 2, [{ uri, isComplete: true, position: 1 } as Checkpoint]), { position: 1, finished: true });
	assert.deepEqual(chapterProgress('t', 2, [{ uri, isComplete: true, position: 0.1 } as Checkpoint]), { position: 0.1, finished: true });
	assert.equal(chapterProgress('t', 3, [{ uri, isComplete: true } as Checkpoint]), null);
});

test('nextStep: the follow-up bar after a group (agents/DESIGN.md "Follow-up bar")', () => {
	const sequence = [
		{ number: 0, status: 'published', isPreview: true },
		{ number: 1, status: 'published' },
		{ number: 2, status: 'draft' },
		{ number: 3, status: 'coming-soon', release: '2025-12-01' }
	];
	const groups = [{ label: 'a' }, { label: 'b' }];
	// more groups follow → the next group
	assert.deepEqual(nextStep({ groups, group: 0, chapter: 1, sequence, member: false }), { kind: 'group', label: 'b' });
	// a following group without a label can't be addressed → the chapter's end
	assert.deepEqual(nextStep({ groups: [{ label: 'a' }, {}], group: 0, chapter: 0, sequence, member: true }), { kind: 'chapter', chapter: 1 });
	// the prologue's end: chapter 1 for a member, the membership case for a guest
	assert.deepEqual(nextStep({ groups: [{}], group: 0, chapter: 0, sequence, member: true }), { kind: 'chapter', chapter: 1 });
	assert.deepEqual(nextStep({ groups: [{}], group: 0, chapter: 0, sequence, member: false }), { kind: 'members', chapter: 1 });
	// a preview chapter needs no membership
	assert.deepEqual(nextStep({ groups: [{}], group: 0, chapter: 0, sequence: [sequence[0], { number: 1, status: 'published', isPreview: true }], member: false }), { kind: 'chapter', chapter: 1 });
	// a draft next: readable in dev only; in production it is coming (no date: not announced) – never skipped
	assert.deepEqual(nextStep({ groups, group: 1, chapter: 1, sequence, member: true, includeDrafts: true }), { kind: 'chapter', chapter: 2 });
	assert.deepEqual(nextStep({ groups, group: 1, chapter: 1, sequence, member: true }), { kind: 'coming-soon', chapter: 2 });
	// coming soon (with its release date)
	assert.deepEqual(nextStep({ groups, group: 1, chapter: 2, sequence, member: true }), { kind: 'coming-soon', chapter: 3, release: '2025-12-01' });
	assert.deepEqual(nextStep({ groups, group: 1, chapter: 2, sequence: [{ number: 2 }, { number: 3, status: 'coming-soon' }], member: true }), { kind: 'coming-soon', chapter: 3 });
	// the last chapter of the season
	assert.deepEqual(nextStep({ groups, group: 1, chapter: 3, sequence, member: true }), { kind: 'season-end' });
	// a chapter the sequence doesn't list (a draft in production): the next listed one after it
	assert.deepEqual(nextStep({ groups: [{}], group: 0, chapter: 2, sequence: [sequence[0], sequence[1], sequence[3]], member: true }), { kind: 'coming-soon', chapter: 3, release: '2025-12-01' });
});

test('the reader history (game state `history`): counts, normalizing older / broken states, the limit', async () => {
	const { HISTORY_LIMIT, historyCount, normalizeHistory, recordHistory, textHistoryKey } = await import('../src/index.ts');
	const key = textHistoryKey('chapter:t:1', 'b1/0', 0);
	assert.equal(key, 'text:chapter:t:1/b1/0#0');
	const h = normalizeHistory(undefined);
	assert.deepEqual(h, {});
	assert.equal(historyCount(h, key), 0);
	assert.deepEqual(recordHistory(h, key, 10), { n: 1, first: 10, last: 10 });
	assert.deepEqual(recordHistory(h, key, 20), { n: 2, first: 10, last: 20 });
	assert.equal(historyCount(h, key), 2);
	assert.deepEqual(normalizeHistory({ a: { n: 3, last: 5 }, b: { n: -1 }, c: 'x', d: null }), { a: { n: 3, first: 5, last: 5 } });
	assert.deepEqual(normalizeHistory([1, 2]), {});
	const many = normalizeHistory({});
	for (let i = 0; i <= HISTORY_LIMIT; i++) recordHistory(many, `k${i}`, i);
	assert.equal(Object.keys(many).length, HISTORY_LIMIT);
	assert.equal(historyCount(many, 'k0'), 0, 'the longest-unseen entry went');
	assert.equal(historyCount(many, `k${HISTORY_LIMIT}`), 1);
});
