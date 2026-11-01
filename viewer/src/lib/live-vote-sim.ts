// #231 draft only: a fake relay for the vote screens' drafts (/account/lab/vote). It speaks the real protocol types
// (`@scutoons/live`, generated from deck/protocol/schema.json) and mirrors the Rust relay's vote (deck/relay/src/vote.rs
// `decide`, session.rs `tick` / `close_vote`): vote.open → vote.tally every 250 ms (conductor; voters too when `show`) →
// vote.result (by the deadline, when a `first` option reaches n, when every connected device has voted, or by Close now /
// Override); vote.hold stops the countdown. A voter gets the tally only after voting (when the vote shows it). A
// simulated crowd casts. Same contract as the real relay (live-vote-link.ts) – both are a `VoteSource`.
import type { Envelope, VoteOpen, VoteResult, VoteTally } from '@scutoons/live';
import type { VoteSource } from './live-vote.ts';

export const TALLY_MS = 250;

export interface Cast {
	option: number;
	group?: string;
}

/** vote.rs `winner`: the most votes; a tie that includes the fallback goes to the fallback, else the first. */
export function winner(counts: number[], fallback: number): number {
	const max = Math.max(0, ...counts);
	if (counts[fallback] === max) return fallback;
	const i = counts.indexOf(max);
	return i < 0 ? fallback : i;
}

export function count(n: number, options: Iterable<number>): number[] {
	const counts = new Array<number>(n).fill(0);
	for (const o of options) if (o >= 0 && o < n) counts[o]++;
	return counts;
}

/** the counts per group (a cast without a group counts under `-`) */
export function groupCounts(n: number, casts: Iterable<Cast>): Record<string, number[]> {
	const out: Record<string, number[]> = {};
	for (const c of casts) (out[c.group ?? '-'] ??= new Array<number>(n).fill(0))[c.option]++;
	return out;
}

export function fallbackOf(open: VoteOpen): number {
	const f = Math.max(0, open.fallback ?? 0);
	return f < open.options.length ? f : 0;
}

/** rule `first`: the first option that reached n (default 1). */
export function reached(open: VoteOpen, counts: number[]): number | undefined {
	if (open.rule !== 'first') return undefined;
	const n = Math.max(1, open.n ?? 1);
	const i = counts.findIndex((c) => c >= n);
	return i < 0 ? undefined : i;
}

/** vote.rs `decide`: override → fallback when nobody voted → the rule. */
export function decide(open: VoteOpen, casts: Iterable<Cast>, override?: number): VoteResult {
	const list = [...casts];
	const n = open.options.length;
	const counts = count(n, list.map((c) => c.option));
	const fallback = fallbackOf(open);
	const groups = groupCounts(n, list);
	const base = { vote: open.vote, counts, ...(Object.keys(groups).length ? { groups } : {}) };
	if (override !== undefined && override >= 0 && override < n) return { ...base, option: override, how: 'override' };
	if (list.length === 0) return { ...base, option: fallback, how: 'fallback' };
	let option: number;
	if (open.rule === 'groups') {
		const byGroup = new Map<string | undefined, number[]>();
		for (const c of list) byGroup.set(c.group, [...(byGroup.get(c.group) ?? []), c.option]);
		const groupVotes = [...byGroup.values()].map((os) => winner(count(n, os), fallback));
		option = winner(count(n, groupVotes), fallback);
	} else {
		option = reached(open, counts) ?? winner(counts, fallback);
	}
	return { ...base, option, how: 'rule' };
}

export interface Clock {
	now(): number;
	/** calls `fn` every `ms`; returns the stop */
	every(ms: number, fn: () => void): () => void;
}

export const realClock: Clock = {
	now: () => Date.now(),
	every: (ms, fn) => {
		const id = setInterval(fn, ms);
		return () => clearInterval(id);
	}
};

/** One simulated group: `size` phones, `lean` = how likely each option is picked (weights). */
export interface CrowdGroup {
	group: string;
	size: number;
	lean: number[];
}

type Listener = (e: Envelope) => void;
interface Sub {
	role: 'conductor' | 'audience';
	group?: string;
	client?: string;
	fn: Listener;
}

/** mulberry32 – a seeded random, so a draft run (and a test) is repeatable */
export function seeded(seed: number): () => number {
	let a = seed >>> 0;
	return () => {
		a = (a + 0x6d2b79f5) >>> 0;
		let t = a;
		t = Math.imul(t ^ (t >>> 15), t | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

export class FakeVoteRelay implements VoteSource {
	private subs: Sub[] = [];
	private seq = 0;
	private vote: { open: VoteOpen; deadline: number; casts: Map<string, Cast>; heldAt: number | null } | null = null;
	private stop: (() => void) | null = null;
	private random: () => number;

	constructor(
		private clock: Clock = realClock,
		private crowd: CrowdGroup[] = [],
		seed = 7,
		/** the connected voting devices: the vote closes when every one has voted (owner 2026-11-01; the Rust relay
		 *  closes only at the deadline / `first` / conductor today – a relay change) */
		private expected = 0
	) {
		this.random = seeded(seed);
	}

	/** a screen listens as the conductor (everything) or as an audience phone in `group`; an open vote reaches it at
	 *  once, like the snapshot's `vote` on join */
	on(role: Sub['role'], fn: Listener, group?: string, client?: string): () => void {
		const sub: Sub = { role, group, client, fn };
		this.subs.push(sub);
		const v = this.vote;
		if (v && (role === 'conductor' || this.isVoter(group))) fn({ v: 1, seq: this.seq, from: 'relay', at: this.clock.now(), kind: 'vote.open', p: v.open });
		return () => (this.subs = this.subs.filter((s) => s !== sub));
	}

	get isOpen(): boolean {
		return this.vote !== null;
	}

	/** conductor → relay: vote.open (a new one replaces an open one, like the relay) */
	open(p: VoteOpen): void {
		this.halt();
		this.vote = { open: p, deadline: this.clock.now() + Math.max(1, p.seconds) * 1000, casts: new Map(), heldAt: null };
		this.emit({ kind: 'vote.open', p }, (s) => s.role === 'conductor' || this.isVoter(s.group));
		this.stop = this.clock.every(TALLY_MS, () => this.tick());
	}

	/** client → relay: vote.cast, one per client, changeable until the close; a non-voter is ignored */
	cast(client: string, option: number, group?: string): void {
		const v = this.vote;
		if (!v || !this.isVoter(group) || option < 0 || option >= v.open.options.length) return;
		v.casts.set(client, { option, group });
		if (reached(v.open, this.counts()) !== undefined || this.allVoted()) this.close();
	}

	/** conductor → relay: vote.close – Close now, or Override with the conductor's option */
	close(override?: number): void {
		const v = this.vote;
		if (!v) return;
		const result = decide(v.open, v.casts.values(), override);
		this.halt();
		this.emit({ kind: 'vote.result', p: result }, () => true);
	}

	/** conductor → relay: vote.hold – the countdown stops where it is (votes still count), `false` runs it on */
	hold(hold: boolean): void {
		const v = this.vote;
		if (!v) return;
		if (hold && v.heldAt === null) v.heldAt = this.clock.now();
		else if (!hold && v.heldAt !== null) {
			v.deadline += this.clock.now() - v.heldAt;
			v.heldAt = null;
		}
	}

	private allVoted(): boolean {
		const v = this.vote;
		return !!v && v.heldAt === null && this.expected > 0 && v.casts.size >= this.expected;
	}

	/** what one client cast (the phone's own mark after a reconnect would come from here) */
	castOf(client: string): number | undefined {
		return this.vote?.casts.get(client)?.option;
	}

	dispose(): void {
		this.halt();
		this.subs = [];
	}

	private halt(): void {
		this.stop?.();
		this.stop = null;
		this.vote = null;
	}

	private counts(): number[] {
		const v = this.vote!;
		return count(v.open.options.length, [...v.casts.values()].map((c) => c.option));
	}

	private isVoter(group?: string): boolean {
		const groups = this.vote?.open.groups;
		return !groups || (group !== undefined && groups.includes(group));
	}

	/** session.rs `tick`: the crowd casts, the vote closes at its deadline (never while held) or when all voted, else the
	 *  tally goes out – to the conductor with the counts per group, to a voter only after they voted */
	private tick(): void {
		const v = this.vote;
		if (!v) return;
		const now = this.clock.now();
		if (v.heldAt === null && now >= v.deadline) return this.close();
		this.crowdCasts(1 - (v.deadline - now) / (v.open.seconds * 1000));
		if (!this.vote || this.allVoted()) return this.close();
		const left = Math.max(0, v.deadline - (v.heldAt ?? now));
		const tally: VoteTally = { vote: v.open.vote, counts: this.counts(), voters: v.casts.size, left, ...(v.heldAt !== null ? { paused: true } : {}) };
		const conductor: VoteTally = { ...tally, groups: groupCounts(v.open.options.length, v.casts.values()), expected: this.expected };
		const show = v.open.show === true;
		this.emit({ kind: 'vote.tally', p: conductor }, (s) => s.role === 'conductor');
		this.emit({ kind: 'vote.tally', p: tally }, (s) => s.role === 'audience' && show && this.isVoter(s.group) && s.client !== undefined && v.casts.has(s.client));
	}

	/** most phones vote early, a few change their mind */
	private crowdCasts(elapsed: number): void {
		const n = this.vote!.open.options.length;
		for (const g of this.crowd) {
			for (let i = 0; i < g.size && this.vote; i++) {
				const id = `${g.group}-${i}`;
				const had = this.vote.casts.has(id);
				const p = had ? 0.004 : 0.03 + 0.12 * (1 - elapsed);
				if (this.random() < p) this.cast(id, this.pick(g.lean, n), g.group);
			}
		}
	}

	private pick(lean: number[], n: number): number {
		const w = Array.from({ length: n }, (_, i) => Math.max(0, lean[i] ?? 0));
		let r = this.random() * w.reduce((a, b) => a + b, 0);
		for (let i = 0; i < n; i++) if ((r -= w[i]) < 0) return i;
		return n - 1;
	}

	private emit(m: Pick<Envelope, 'kind' | 'p'>, to: (s: Sub) => boolean): void {
		const reach = this.subs.filter(to);
		if (!reach.length) return; // the relay stamps only what it delivers
		const e = { v: 1, seq: ++this.seq, from: 'relay', at: this.clock.now(), ...m } as Envelope;
		for (const s of reach) s.fn(e);
	}
}
