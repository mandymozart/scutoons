// The shared views (server/schema/api/network-view.vectors.json) as the TS types read them; the Rust side round-trips the same file.
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { View } from '../src/network-view.ts';

const file = JSON.parse(readFileSync(new URL('../../../server/schema/api/network-view.vectors.json', import.meta.url), 'utf8')) as { views: View[] };

test('every scope the projectors have so far has a vector', () => {
	assert.deepEqual(file.views.map((v) => v.scope), ['story', 'rig', 'network']);
});

test('every edge joins ports that exist, out to in, with the edge’s signal (a far end takes any)', () => {
	for (const view of file.views) {
		const port = (node: string, id: string) => view.nodes.find((n) => n.id === node)?.ports.find((p) => p.id === id);
		for (const e of view.edges) {
			const from = port(e.from.node, e.from.port), to = port(e.to.node, e.to.port);
			assert.ok(from && to, `${view.context} ${e.id}: a pin is missing`);
			assert.equal(from.dir, 'out', `${view.context} ${e.id}`);
			assert.equal(to.dir, 'in', `${view.context} ${e.id}`);
			for (const p of [from, to]) assert.ok(p.signal === e.signal || p.signal === 'far', `${view.context} ${e.id}: ${p.signal} ≠ ${e.signal}`);
		}
	}
});

test('every stub names an edge of its view, and the levels end at the context', () => {
	for (const view of file.views) {
		for (const s of view.stubs) assert.ok(view.edges.some((e) => e.id === s.edge), `${view.context}: stub ${s.edge}`);
		assert.equal(view.levels.at(-1)?.address, view.context);
	}
});
