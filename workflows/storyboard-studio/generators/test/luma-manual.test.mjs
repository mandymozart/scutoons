// "Luma – by hand" (luma-manual.mjs): transport 'manual', always configured, free, one manual model that takes the
// prompt; it never runs – a job sent anyway is refused before it exists, writes nothing and spends nothing.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createManager } from '../jobs.mjs';
import { createLumaManual } from '../luma-manual.mjs';

const job = { generator: 'luma-manual', model: 'manual', toon: 'catpope', ch: 's1c3', shot: 'sb-1-1', prompt: 'a shot' };

test('listed as by hand (transport manual), configured and free, with one manual model taking the prompt', async () => {
	const m = createManager({ generators: [createLumaManual()], out: mkdtempSync(join(tmpdir(), 'sb-luma-')) });
	const [g] = await m.list();
	assert.equal(g.id, 'luma-manual');
	assert.equal(g.transport, 'manual');
	assert.equal(g.configured, true);
	assert.equal(g.paid, false);
	assert.match(g.note, /lumalabs\.ai/);
	assert.match(g.note, /Variants/);
	assert.deepEqual(g.models.map(({ id, mode, inputs }) => ({ id, mode, inputs })), [{ id: 'manual', mode: 'manual', inputs: ['prompt'] }]);
});

test('the estimate is free', () => {
	const m = createManager({ generators: [createLumaManual()], out: mkdtempSync(join(tmpdir(), 'sb-luma-')) });
	const e = m.estimate(job);
	assert.equal(e.paid, false);
	assert.equal(e.model, 'manual');
	assert.equal(e.usd, 0);
	assert.equal(e.credits, 0);
	assert.equal(e.error, undefined);
});

test('never started (no API): a job sent anyway is refused before it exists, writes nothing, spends nothing', async () => {
	await assert.rejects(createLumaManual().run({}, {}), /no API/);
	const out = mkdtempSync(join(tmpdir(), 'sb-luma-'));
	const m = createManager({ generators: [createLumaManual()], out });
	await assert.rejects(m.start(job), /by hand – nothing to send/);
	assert.deepEqual(m.jobs(), []);
	assert.equal(existsSync(join(out, 's1c3')), false);
	assert.deepEqual(m.spend(), { usd: 0, credits: 0, jobs: 0 });
});
