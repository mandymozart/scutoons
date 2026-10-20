// The euro rate (rate.mjs): the ECB file parsed, the fallback when nothing answers, the cache, at most one fetch a day,
// never waiting for it, the env override. No network: the fetch is injected.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createRate, ECB_URL, FALLBACK, parseEcb } from '../rate.mjs';

// the shape of eurofxref-daily.xml (shortened)
const XML = `<?xml version="1.0" encoding="UTF-8"?>
<gesmes:Envelope xmlns:gesmes="http://www.gesmes.org/xml/2002-08-01" xmlns="http://www.ecb.int/vocabulary/2002-08-01/eurofxref">
	<gesmes:subject>Reference rates</gesmes:subject>
	<Cube>
		<Cube time='2026-10-20'>
			<Cube currency='JPY' rate='177.63'/>
			<Cube currency='USD' rate='1.1250'/>
			<Cube currency='GBP' rate='0.84205'/>
		</Cube>
	</Cube>
</gesmes:Envelope>`;
const tmp = () => join(mkdtempSync(join(tmpdir(), 'sb-rate-')), '.cache', 'eur-usd.json');
const offline = () => { const f = async () => { f.calls++; throw new Error('offline'); }; f.calls = 0; return f; };
const ecb = (xml = XML) => { const f = async (url) => { f.calls++; f.url = url; return new Response(xml, { status: 200 }); }; f.calls = 0; return f; };

test('parseEcb reads the USD rate and its date; anything else is null', () => {
	assert.deepEqual(parseEcb(XML), { usdPerEur: 1.125, date: '2026-10-20' });
	assert.deepEqual(parseEcb(`<Cube time="2026-10-19"><Cube currency="USD" rate="1.1225"/></Cube>`), { usdPerEur: 1.1225, date: '2026-10-19' });
	assert.equal(parseEcb('<Cube time="2026-10-19"><Cube currency="GBP" rate="0.84"/></Cube>'), null);
	assert.equal(parseEcb('<html>maintenance</html>'), null);
	assert.equal(parseEcb(''), null);
	assert.equal(parseEcb(undefined), null);
});

test('offline and no cache: the fixed fallback, marked as such – get() never throws or waits', async () => {
	const fetchImpl = offline();
	const r = createRate({ cacheFile: tmp(), env: {}, fetchImpl });
	const first = r.get();
	assert.deepEqual(first, { eurPerUsd: 0.890869, usdPerEur: FALLBACK.usdPerEur, date: FALLBACK.date, source: 'fallback' });
	await r.settled();
	assert.equal(fetchImpl.calls, 1);
	assert.equal(r.get().source, 'fallback');
	assert.equal(fetchImpl.calls, 1, 'a failed fetch is not tried again at once');
});

test('the ECB rate: fetched in the background, cached in memory and in the file, served from the cache when offline', async () => {
	const file = tmp();
	const fetchImpl = ecb();
	const r = createRate({ cacheFile: file, env: {}, fetchImpl });
	assert.equal(r.get().source, 'fallback', 'the first request does not wait for the fetch');
	await r.settled();
	assert.equal(fetchImpl.url, ECB_URL);
	assert.deepEqual(r.get(), { eurPerUsd: 0.888889, usdPerEur: 1.125, date: '2026-10-20', source: 'ecb' });
	assert.ok(existsSync(file));
	assert.equal(JSON.parse(readFileSync(file, 'utf8')).usdPerEur, 1.125);
	// a restart while offline: the cached rate, not the fallback
	const again = createRate({ cacheFile: file, env: {}, fetchImpl: offline() });
	assert.equal(again.get().date, '2026-10-20');
	assert.equal(again.get().source, 'ecb');
});

test('at most one fetch a day; a failed or broken answer keeps the last rate', async () => {
	let t = Date.parse('2026-10-20T10:00:00Z');
	const fetchImpl = ecb();
	const r = createRate({ cacheFile: tmp(), env: {}, fetchImpl, now: () => t });
	r.get();
	await r.settled();
	r.get(); r.get();
	t += 23 * 60 * 60 * 1000;
	r.get();
	await r.settled();
	assert.equal(fetchImpl.calls, 1);
	t += 2 * 60 * 60 * 1000;
	const broken = ecb('<html>maintenance</html>');
	const r2 = createRate({ cacheFile: null, env: {}, fetchImpl: broken, now: () => t });
	r2.get();
	await r2.settled();
	assert.equal(r2.get().source, 'fallback');
	r.get();
	await r.settled();
	assert.equal(fetchImpl.calls, 2, 'a day later it is fetched again');
});

test('STUDIO_EUR_PER_USD overrides it all, without a fetch', () => {
	const fetchImpl = offline();
	const r = createRate({ cacheFile: tmp(), env: { STUDIO_EUR_PER_USD: '0.9' }, fetchImpl });
	assert.deepEqual(r.get(), { eurPerUsd: 0.9, usdPerEur: 1.111111, date: null, source: 'env' });
	assert.equal(fetchImpl.calls, 0);
	assert.equal(createRate({ cacheFile: null, env: { STUDIO_EUR_PER_USD: '0,86' }, fetchImpl }).get().eurPerUsd, 0.86, 'a German decimal comma');
	assert.equal(createRate({ cacheFile: null, env: { STUDIO_EUR_PER_USD: 'x' }, fetchImpl }).get().source, 'fallback');
});
