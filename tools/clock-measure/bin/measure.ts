// `npm run clock:measure -- <recording.wav> [--bound 5]` (#321): how far apart the native deck (1 kHz click) and the
// browser deck (2.5 kHz click) clicked in one recording – the logic is src/measure.ts.
import { readFileSync } from 'node:fs';
import { measure } from '../src/measure.ts';

const args = process.argv.slice(2);
const file = args.find((a) => !a.startsWith('--'));
const bound = Number(args[args.indexOf('--bound') + 1]) || 5;
if (!file) {
	console.error('usage: npm run clock:measure -- <recording.wav> [--bound 5]');
	process.exit(2);
}
const r = measure(readFileSync(file), { bound });
const ms = (x: number) => `${x.toFixed(2)} ms`;
console.log(`clicks: native ${r.a}, browser ${r.b}, paired ${r.pairs}`);
console.log(`native − browser: median ${ms(r.median)}, mean ${ms(r.mean)}; |offset| p95 ${ms(r.p95)}, max ${ms(r.max)}`);
console.log(r.within ? `within ±${bound} ms (p95)` : `NOT within ±${bound} ms (p95)`);
process.exit(r.pairs && r.within ? 0 : 1);
