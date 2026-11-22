// The story's projector for the Spores tab (lib/spores/story.ts): a flow level as a weave of the spore contract.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { FlowEdge, FlowNode } from '@scutoons/content';
import { storyWeave } from '../src/lib/spores/story.ts';

const nodes: FlowNode[] = [
	{ id: 'g0', kind: 'group', status: 'reachable', name: 'Arrival', count: 3 },
	{ id: 'c0', kind: 'choices', status: 'reachable', count: 2 },
	{ id: 'g1', kind: 'group', status: 'never', name: 'Wait' },
	{ id: 'x0', kind: 'exit', status: 'reachable', target: { type: 'xr', value: 'dune-ar' } as FlowNode['target'] }
];
const edges: FlowEdge[] = [
	{ id: 'e0', from: 'g0', to: 'c0', kind: 'then', status: 'reachable' },
	{ id: 'e1', from: 'c0', to: 'g1', kind: 'choice', label: 'wait', status: 'reachable' },
	{ id: 'e2', from: 'c0', to: 'x0', kind: 'jump', status: 'reachable' }
];
const at = { context: 'chapter:sandbox/2', levels: [{ address: 'network', label: 'SCUTOONS' }, { address: 'chapter:sandbox/2', label: '2' }] };

test('every box is a spore and every way a thread between existing ports', () => {
	const w = storyWeave({ nodes, edges }, at);
	assert.deepEqual(w.spores.map((s) => [s.id, s.family]), [['g0', 'container'], ['c0', 'rule'], ['g1', 'container'], ['x0', 'far']]);
	for (const t of w.threads) {
		const from = w.spores.find((s) => s.id === t.from.spore)?.ports.find((p) => p.id === t.from.port && p.dir === 'out');
		const to = w.spores.find((s) => s.id === t.to.spore)?.ports.find((p) => p.id === t.to.port && p.dir === 'in');
		assert.ok(from && to, t.id);
	}
});

test('a choice carries its text, a jump out is a scene thread with a stub named by where it goes', () => {
	const w = storyWeave({ nodes, edges }, at);
	assert.equal(w.threads.find((t) => t.id === 'e1')?.label, 'wait');
	const jump = w.threads.find((t) => t.id === 'e2');
	assert.deepEqual([jump?.signal, jump?.crosses], ['scene', 'navigation']);
	assert.deepEqual(w.stubs, [{ thread: 'e2', side: 'out', far: 'xr:dune-ar', label: '→ xr:dune-ar' }]);
});

test('a box that can never be reached is a problem at it', () => {
	const w = storyWeave({ nodes, edges }, at);
	assert.deepEqual(w.problems, [{ at: 'g1', severity: 'warning', text: 'never' }]);
	assert.equal(w.levels.at(-1)?.address, w.context);
});
