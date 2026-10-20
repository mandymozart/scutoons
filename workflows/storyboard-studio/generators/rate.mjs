// The euro rate (owner 2026-10-20: "preise in euro"): every price the studio shows is in euro, converted at the ECB's
// euro reference rate; the money inside – the ledger, the job records, the confirm check of POST /api/generate – stays
// in the provider's currency (USD), only the display converts (src/lib/generate.svelte.js euro()). README "Generators".
//
// Source: the ECB's daily reference rates (USD per EUR, published on working days around 16:00 CET) → EUR per USD = 1 /
// that. Fetched at most once a day (a failed fetch is tried again after an hour at the earliest), always in the
// background: get() never waits – it serves the last known rate (memory, else .cache/eur-usd.json – gitignored), else
// the fixed FALLBACK, marked as such. STUDIO_EUR_PER_USD=<EUR per USD> overrides it all (no fetch).

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';

export const ECB_URL = 'https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml';
/** the fixed fallback when neither the ECB nor the cache has answered: the ECB reference rate of 2026-10-19,
 *  1 EUR = 1.1225 USD → 1 USD = 0.8909 € */
export const FALLBACK = { usdPerEur: 1.1225, date: '2026-10-19' };
const DAY = 24 * 60 * 60 * 1000;
const RETRY = 60 * 60 * 1000;

/** the ECB's daily XML → { usdPerEur, date } – null when it holds no USD rate */
export function parseEcb(xml) {
	const text = String(xml ?? '');
	const date = /<Cube\s+time=['"](\d{4}-\d{2}-\d{2})['"]/.exec(text)?.[1];
	const usdPerEur = Number(/<Cube\s+currency=['"]USD['"]\s+rate=['"]([\d.]+)['"]/.exec(text)?.[1]);
	return date && usdPerEur > 0 ? { usdPerEur, date } : null;
}

const round = (x) => Math.round(x * 1e6) / 1e6;
/** what the API serves: { eurPerUsd, usdPerEur, date, source: 'ecb' | 'fallback' | 'env' } */
const served = ({ usdPerEur, date }, source) => ({ eurPerUsd: round(1 / usdPerEur), usdPerEur, date, source });

export function createRate({ cacheFile = null, env = process.env, fetchImpl = fetch, now = () => Date.now(), timeoutMs = 10000 } = {}) {
	/** the last ECB rate known: { usdPerEur, date, fetched (ms) } */
	let known = null;
	let tried = -Infinity;
	let pending = null;
	try {
		const c = cacheFile && existsSync(cacheFile) ? JSON.parse(readFileSync(cacheFile, 'utf8')) : null;
		if (c?.usdPerEur > 0 && /^\d{4}-\d{2}-\d{2}$/.test(c.date)) known = { usdPerEur: c.usdPerEur, date: c.date, fetched: Date.parse(c.fetched) || 0 };
	} catch {}

	async function fetchRate() {
		tried = now();
		try {
			const r = await fetchImpl(ECB_URL, { signal: AbortSignal.timeout(timeoutMs) });
			if (!r.ok) throw new Error(`ECB ${r.status}`);
			const hit = parseEcb(await r.text());
			if (!hit) throw new Error('no USD rate in the ECB file');
			known = { ...hit, fetched: now() };
			if (cacheFile) {
				mkdirSync(dirname(cacheFile), { recursive: true });
				writeFileSync(cacheFile, JSON.stringify({ usdPerEur: hit.usdPerEur, date: hit.date, fetched: new Date(known.fetched).toISOString(), source: ECB_URL }, null, '\t') + '\n');
			}
		} catch {
			// offline or a changed file: the last known rate (or the fallback) stays – tried again after RETRY
		}
	}

	function due() {
		const t = now();
		return !pending && (!known || t - known.fetched >= DAY) && t - tried >= RETRY;
	}

	return {
		/** the rate to show now – never waits; starts a background fetch when one is due */
		get() {
			const fixed = Number(String(env.STUDIO_EUR_PER_USD ?? '').trim().replace(',', '.')); // "0.86" or "0,86"
			if (env.STUDIO_EUR_PER_USD && fixed > 0) return { eurPerUsd: fixed, usdPerEur: round(1 / fixed), date: null, source: 'env' };
			if (due()) pending = fetchRate().finally(() => { pending = null; });
			return known ? served(known, 'ecb') : served(FALLBACK, 'fallback');
		},
		/** for tests: the background fetch running now, if any */
		settled: () => pending ?? Promise.resolve()
	};
}
