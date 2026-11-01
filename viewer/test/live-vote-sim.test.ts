// #231: the fake relay behind the vote drafts (src/lib/live-vote-sim.ts) – the same outcomes as deck/relay/src/vote.rs.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { Envelope, VoteOpen, VoteTally } from '@scutoons/live';
import { decide, FakeVoteRelay, TALLY_MS, type Clock } from '../src/lib/live-vote-sim.ts';

const open = (rule: VoteOpen['rule'], more: Partial<VoteOpen> = {}): VoteOpen => ({
	vote: 'v1',
	options: ['left', 'right', 'stay'],
	rule,
	seconds: 20,
	fallback: 2,
	...more
});

class ManualClock implements Clock {
	t = 1000;
	fns: { ms: number; fn: () => void; next: number }[] = [];
	now = () => this.t;
	every = (ms: number, fn: () => void) => {
		const entry = { ms, fn, next: this.t + ms };
		this.fns.push(entry);
		return () => (this.fns = this.fns.filter((f) => f !== entry));
	};
	advance(ms: number) {
		const end = this.t + ms;
		for (;;) {
			const due = this.fns.filter((f) => f.next <= end).sort((a, b) => a.next - b.next)[0];
			if (!due) break;
			this.t = due.next;
			due.next += due.ms;
			due.fn();
		}
		this.t = end;
	}
}

test('nobody voted → the fallback', () => {
	const r = decide(open('majority'), []);
	assert.deepEqual([r.option, r.how, r.counts], [2, 'fallback', [0, 0, 0]]);
});

test('majority; a tie with the fallback goes to the fallback, else the first', () => {
	const c = (o: number) => ({ option: o, group: 'g' });
	assert.equal(decide(open('majority'), [c(0), c(1), c(1)]).option, 1);
	assert.equal(decide(open('majority'), [c(0), c(1)]).option, 0);
	assert.equal(decide(open('majority'), [c(0), c(2)]).option, 2);
});

test('groups: each group counts once', () => {
	const casts = [...Array.from({ length: 10 }, () => ({ option: 0, group: 'left' })), { option: 1, group: 'right' }, { option: 1, group: 'balcony' }];
	const r = decide(open('groups'), casts);
	assert.deepEqual([r.option, r.how, r.counts], [1, 'rule', [10, 2, 0]]);
});

test('override wins over the rule', () => {
	const r = decide(open('majority'), [{ option: 0 }], 1);
	assert.deepEqual([r.option, r.how], [1, 'override']);
});

test('tally every 250 ms to the conductor, with what is left; closes itself at the deadline', () => {
	const clock = new ManualClock();
	const relay = new FakeVoteRelay(clock);
	const seen: Envelope[] = [];
	relay.on('conductor', (e) => seen.push(e));
	relay.open(open('majority', { seconds: 2 }));
	relay.cast('a', 1);
	clock.advance(TALLY_MS);
	const tally = seen.find((e) => e.kind === 'vote.tally');
	assert.ok(tally && tally.kind === 'vote.tally');
	assert.deepEqual(tally.p, { vote: 'v1', counts: [0, 1, 0], voters: 1, left: 1750, groups: { '-': [0, 1, 0] }, expected: 0 });
	clock.advance(2000);
	const result = seen.at(-1)!;
	assert.equal(result.kind, 'vote.result');
	assert.equal(relay.isOpen, false);
	assert.deepEqual(seen.map((e) => e.seq), seen.map((_, i) => i + 1));
});

test('first to n closes when an option reaches n', () => {
	const relay = new FakeVoteRelay(new ManualClock());
	let result: Envelope | undefined;
	relay.on('conductor', (e) => e.kind === 'vote.result' && (result = e));
	relay.open(open('first', { n: 2 }));
	relay.cast('a', 0);
	relay.cast('b', 0);
	assert.ok(result && result.kind === 'vote.result');
	assert.deepEqual([result.p.option, result.p.how], [0, 'rule']);
});

test('a vote can change until the close; only voters of the vote hear of it', () => {
	const clock = new ManualClock();
	const relay = new FakeVoteRelay(clock);
	const phone: string[] = [];
	const other: string[] = [];
	relay.on('audience', (e) => phone.push(e.kind), 'balcony', 'me');
	relay.on('audience', (e) => other.push(e.kind), 'stalls', 'x');
	relay.open(open('majority', { groups: ['balcony'] }));
	relay.cast('me', 0, 'balcony');
	relay.cast('me', 1, 'balcony');
	relay.cast('x', 0, 'stalls'); // not a voter
	assert.equal(relay.castOf('me'), 1);
	clock.advance(TALLY_MS);
	relay.close();
	assert.deepEqual(phone, ['vote.open', 'vote.result']);
	assert.deepEqual(other, ['vote.result']);
});

test('a voter gets the tally only after voting, and only when the vote shows it', () => {
	const clock = new ManualClock();
	const relay = new FakeVoteRelay(clock);
	const kinds = (show: boolean) => {
		const got: string[] = [];
		const off = relay.on('audience', (e) => got.push(e.kind), 'g', 'me');
		relay.open(open('majority', { show }));
		clock.advance(TALLY_MS);
		const before = got.filter((k) => k === 'vote.tally').length;
		relay.cast('me', 0, 'g');
		clock.advance(TALLY_MS);
		off();
		return [before, got.filter((k) => k === 'vote.tally').length];
	};
	assert.deepEqual(kinds(true), [0, 1], 'nothing before the vote, then the totals');
	assert.deepEqual(kinds(false), [0, 0], 'a vote that does not show it never sends it');
});

test('hold stops the countdown; a held vote waits even when everyone voted; run on closes it', () => {
	const clock = new ManualClock();
	const relay = new FakeVoteRelay(clock, [], 7, 1);
	const tallies: VoteTally[] = [];
	const results: Envelope[] = [];
	relay.on('conductor', (e) => (e.kind === 'vote.tally' ? tallies.push(e.p) : e.kind === 'vote.result' && results.push(e)));
	relay.open(open('majority', { seconds: 10 }));
	clock.advance(2000);
	relay.hold(true);
	clock.advance(30_000);
	assert.equal(relay.isOpen, true, 'held: no deadline');
	const last = tallies.at(-1)!;
	assert.deepEqual([last.paused, last.left], [true, 8000]);
	relay.cast('a', 1, 'g'); // the only device voted
	assert.equal(relay.isOpen, true, 'held');
	relay.hold(false);
	clock.advance(TALLY_MS);
	assert.equal(relay.isOpen, false);
	assert.equal(results.length, 1);
});

test('the crowd casts by its lean (seeded, repeatable)', () => {
	const run = () => {
		const clock = new ManualClock();
		const relay = new FakeVoteRelay(clock, [{ group: 'g', size: 40, lean: [3, 1] }], 3);
		let last: number[] = [];
		relay.on('conductor', (e) => e.kind === 'vote.tally' && (last = e.p.counts));
		relay.open({ vote: 'v', options: ['yes', 'no'], rule: 'majority', seconds: 10 });
		clock.advance(5000);
		return last;
	};
	const a = run();
	assert.deepEqual(a, run());
	assert.ok(a[0] > a[1] && a[0] + a[1] > 20);
});

test('a screen that subscribes while a vote is open gets it at once (the snapshot)', () => {
	const relay = new FakeVoteRelay(new ManualClock());
	relay.open(open('majority', { groups: ['balcony'] }));
	const got: string[] = [];
	relay.on('audience', (e) => got.push(e.kind), 'balcony');
	relay.on('audience', (e) => got.push(`other:${e.kind}`), 'stalls');
	assert.deepEqual(got, ['vote.open']);
});

test('the vote closes when every connected device has voted', () => {
	const relay = new FakeVoteRelay(new ManualClock(), [], 7, 2);
	const results: Envelope[] = [];
	relay.on('conductor', (e) => e.kind === 'vote.result' && results.push(e));
	relay.open(open('majority'));
	relay.cast('a', 0, 'g');
	assert.equal(results.length, 0);
	relay.cast('a', 1, 'g'); // a change is not a second device
	assert.equal(results.length, 0);
	relay.cast('b', 1, 'g');
	const r = results[0];
	assert.ok(r && r.kind === 'vote.result' && r.p.option === 1);
	assert.equal(relay.isOpen, false);
});

test('the result carries the counts per group', () => {
	const r = decide(open('groups'), [
		{ option: 0, group: 'left' },
		{ option: 1, group: 'left' },
		{ option: 1, group: 'right' }
	]);
	assert.deepEqual(r.groups, { left: [1, 1, 0], right: [0, 1, 0] });
	assert.equal(decide(open('majority'), []).groups, undefined);
});
