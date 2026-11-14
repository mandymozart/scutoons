// The Live vote demo (#231): phones that vote – something to show and test /live/vote and the deck editor's Vote sheet
// (Perform, #533) with. Thin CLI; the bots' logic is viewer/src/lib/live-vote-demo.ts (tested), the link is
// viewer/src/lib/live-vote-link.ts.
//
//   npm run live:vote-demo -- [--relay=ws://127.0.0.1:7700] [--code=DEMO] [--phones=12]
//
//   phones  per group (balkon, parkett, rang) – bots that vote on every vote that opens, evenly
// Start a relay first: `cargo run -p relay` in deck/ (listens on 127.0.0.1:7700), then open votes in the deck editor:
// `cargo run -p deck-editor -- --workspace perform --relay ws://127.0.0.1:7700/live/DEMO`, the Vote sheet.
import { GROUPS, changesMind, pickOption, voteDelay } from '../../viewer/src/lib/live-vote-demo.ts';
import { readFileSync } from 'node:fs';
import { loadRelay } from '../../shared/live-deck/src/client.ts';
import { RelayVote } from '../../viewer/src/lib/live-vote-link.ts';

const arg = (name: string, fallback = ''): string => process.argv.find((a) => a.startsWith(`--${name}=`))?.slice(name.length + 3) ?? fallback;

const base = arg('relay', 'ws://127.0.0.1:7700').replace(/\/+$/, '');
const code = arg('code', 'DEMO');
const phones = Math.max(1, Number(arg('phones', '12')));
const relay = `${base}/live/${code}`;
// #366: the relay client is the core's (relay-web; `npm run build:wasm -w shared/live-deck -- --relay`)
const wasmDir = new URL('../../shared/live-deck/wasm/', import.meta.url);
const core = await loadRelay(wasmDir.href, readFileSync(new URL('relay_web_bg.wasm', wasmDir)));

function phone(group: string, n: number) {
	const link = new RelayVote({ relay, group, nick: `${group}-${n}`, core });
	let current = '';
	link.on('audience', (e) => {
		if (e.kind === 'vote.result') current = '';
		if (e.kind !== 'vote.open') return;
		const open = e.p;
		current = open.vote;
		const even = open.options.map(() => 1);
		const choose = () => current === open.vote && link.cast('', pickOption(even, open.options.length, Math.random()));
		setTimeout(choose, voteDelay(open.seconds, Math.random()));
		if (changesMind(Math.random())) setTimeout(choose, voteDelay(open.seconds, Math.random()) + 1500);
	});
	return link;
}

const bots = GROUPS.flatMap((g) => Array.from({ length: phones }, (_, i) => phone(g, i + 1)));
console.log(`${bots.length} phones in ${GROUPS.join(', ')} → ${relay}`);
console.log(`open votes in the deck editor (Perform → Vote, --relay ${relay}) – Ctrl+C stops the phones`);
await new Promise(() => {});
