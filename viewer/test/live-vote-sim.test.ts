// #231: the fake relay behind the vote drafts (src/lib/live-vote-sim.ts) – the same outcomes as deck/relay/src/vote.rs.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { Envelope, VoteOpen } from '@scutoons/live';
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
	assert.deepEqual(tally.p, { vote: 'v1', counts: [0, 1, 0], voters: 1, left: 1750 });
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

test('a vote can change until the close; the audience gets the tally only when the vote shows it', () => {
	const clock = new ManualClock();
	const relay = new FakeVoteRelay(clock);
	const phone: string[] = [];
	const other: string[] = [];
	relay.on('audience', (e) => phone.push(e.kind), 'balcony');
	relay.on('audience', (e) => other.push(e.kind), 'stalls');
	relay.open(open('majority', { groups: ['balcony'] }));
	relay.cast('me', 0, 'balcony');
	relay.cast('me', 1, 'balcony');
	relay.cast('x', 0, 'stalls'); // not a voter
	assert.equal(relay.castOf('me'), 1);
	clock.advance(TALLY_MS);
	relay.close();
	assert.deepEqual(phone, ['vote.open', 'vote.result']);
	assert.deepEqual(other, ['vote.result']);
	relay.open(open('majority', { show: true }));
	clock.advance(TALLY_MS);
	assert.equal(phone.at(-1), 'vote.tally');
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

test('groupCounts: the draft-only breakdown per group', () => {
	const relay = new FakeVoteRelay(new ManualClock());
	relay.open(open('groups'));
	relay.cast('a', 0, 'left');
	relay.cast('b', 1, 'left');
	relay.cast('c', 1, 'right');
	assert.deepEqual(relay.groupCounts(), { left: [1, 1, 0], right: [0, 1, 0] });
});
