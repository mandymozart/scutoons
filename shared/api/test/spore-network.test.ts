// The shared weaves (server/schema/api/spore-network.vectors.json) as the TS types read them; the Rust side round-trips the same file.
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { Weave } from '../src/spore-network.ts';

const file = JSON.parse(readFileSync(new URL('../../../server/schema/api/spore-network.vectors.json', import.meta.url), 'utf8')) as { weaves: Weave[] };

test('every scope the projectors have so far has a weave', () => {
	assert.deepEqual(file.weaves.map((w) => w.scope), ['story', 'rig', 'network', 'xr']);
});

test('every thread joins ports that exist, out to in, with the thread’s signal (a far end takes any)', () => {
	for (const weave of file.weaves) {
		const port = (spore: string, id: string) => weave.spores.find((s) => s.id === spore)?.ports.find((p) => p.id === id);
		for (const t of weave.threads) {
			const from = port(t.from.spore, t.from.port), to = port(t.to.spore, t.to.port);
			assert.ok(from && to, `${weave.context} ${t.id}: an end is missing`);
			assert.equal(from.dir, 'out', `${weave.context} ${t.id}`);
			assert.equal(to.dir, 'in', `${weave.context} ${t.id}`);
			for (const p of [from, to]) assert.ok(p.signal === t.signal || p.signal === 'far', `${weave.context} ${t.id}: ${p.signal} ≠ ${t.signal}`);
		}
	}
});

test('every stub names a thread of its weave, and the levels end at the context', () => {
	for (const weave of file.weaves) {
		for (const s of weave.stubs) assert.ok(weave.threads.some((t) => t.id === s.thread), `${weave.context}: stub ${s.thread}`);
		assert.equal(weave.levels.at(-1)?.address, weave.context);
	}
});
