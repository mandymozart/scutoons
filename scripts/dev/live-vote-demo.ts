// The Live vote demo (#231): phones that vote, and optionally a conductor that plays the sandbox's votes one after
// another – something to show and test /live/vote and /live/conductor with. Thin CLI; the data and the bots' logic are
// viewer/src/lib/live-vote-demo.ts (tested), the link is viewer/src/lib/live-vote-link.ts.
//
//   npm run live:vote-demo -- [--relay=ws://127.0.0.1:7700] [--code=DEMO] [--phones=12] [--conductor] [--loop]
//
//   phones      per group (balkon, parkett, rang) – bots that vote on every vote that opens, by the scenario's lean
//   --conductor also play the 4 scenarios as the conductor (key "demo"; a relay with a secret needs a real token: --key=)
//   without it  you open the votes yourself on /live/conductor/<code>?key=demo (the bots vote on yours too, evenly)
// Start a relay first: `cargo run -p relay` in deck/ (listens on 127.0.0.1:7700; dev mode takes any key).
import type { VoteOpen, VoteResult } from '@scutoons/live';
import { changesMind, crowdFor, GROUPS, pickOption, SCENARIOS, voteDelay } from '../../viewer/src/lib/live-vote-demo.ts';
import { readFileSync } from 'node:fs';
import { loadRelay } from '../../shared/live-deck/src/client.ts';
import { RelayVote } from '../../viewer/src/lib/live-vote-link.ts';

const arg = (name: string, fallback = ''): string => process.argv.find((a) => a.startsWith(`--${name}=`))?.slice(name.length + 3) ?? fallback;
const flag = (name: string): boolean => process.argv.includes(`--${name}`);

const base = arg('relay', 'ws://127.0.0.1:7700').replace(/\/+$/, '');
const code = arg('code', 'DEMO');
const phones = Math.max(1, Number(arg('phones', '12')));
const relay = `${base}/live/${code}`;
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
// #366: the relay client is the core's (relay-web; `npm run build:wasm -w shared/live-deck -- --relay`)
const wasmDir = new URL('../../shared/live-deck/wasm/', import.meta.url);
const core = await loadRelay(wasmDir.href, readFileSync(new URL('relay_web_bg.wasm', wasmDir)));

/** the lean per group for the vote that is open now (a vote of ours: the scenario's; yours: even) */
let lean: Record<string, number[]> = {};
const leanFor = (open: VoteOpen) => SCENARIOS.find((s) => s.open.vote === open.vote)?.crowd ?? Object.fromEntries(GROUPS.map((g) => [g, open.options.map(() => 1)]));

function phone(group: string, n: number) {
	const link = new RelayVote({ relay, role: 'audience', group, nick: `${group}-${n}`, core });
	let current = '';
	link.on('audience', (e) => {
		if (e.kind === 'vote.result') current = '';
		if (e.kind !== 'vote.open') return;
		const open = e.p;
		current = open.vote;
		const mine = lean[group] ?? leanFor(open)[group];
		if (!mine) return; // a silent group in this scenario
		const choose = () => current === open.vote && link.cast('', pickOption(mine, open.options.length, Math.random()));
		setTimeout(choose, voteDelay(open.seconds, Math.random()));
		if (changesMind(Math.random())) setTimeout(choose, voteDelay(open.seconds, Math.random()) + 1500);
	});
	return link;
}

const bots = GROUPS.flatMap((g) => Array.from({ length: phones }, (_, i) => phone(g, i + 1)));
console.log(`${bots.length} phones in ${GROUPS.join(', ')} → ${relay}`);

if (flag('conductor')) {
	const conductor = new RelayVote({ relay, role: 'conductor', token: arg('key', 'demo'), core, onStatus: (s) => console.log('conductor', s.state) });
	let result: ((r: VoteResult) => void) | null = null;
	conductor.on('conductor', (e) => {
		if (e.kind === 'vote.result') result?.(e.p);
		else if (e.kind === 'error') console.log('relay:', e.p.code, e.p.message);
	});
	await wait(2000);
	do {
		for (const s of SCENARIOS) {
			lean = Object.fromEntries(crowdFor(s.crowd, phones).map((g) => [g.id, g.lean]));
			console.log(`\n▶ ${s.title} (${s.open.rule}, ${s.open.seconds} s)`);
			const done = new Promise<VoteResult>((r) => (result = r));
			conductor.open(s.open);
			const r = await done;
			console.log(`  → ${s.open.options[r.option]} (${r.how})  counts ${r.counts.join(' : ')}`);
			await wait(6000);
		}
	} while (flag('loop'));
	process.exit(0);
} else {
	console.log(`open votes on /live/conductor/${code}?key=demo&relay=${base} – Ctrl+C stops the phones`);
	await new Promise(() => {});
}
