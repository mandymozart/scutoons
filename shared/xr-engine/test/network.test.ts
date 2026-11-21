import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { projectXr } from '../src/network.ts';
import { doc } from './network-fixture.ts';

test('the experience level: its scenes, the goto between them, the plugin, in the contract', () => {
	const view = projectXr(doc, 'experience:demo');
	assert.equal(view.scope, 'xr');
	assert.equal(view.context, 'experience:demo');
	assert.deepEqual(view.levels, [{ address: 'experience:demo', label: 'Demo' }]);
	const scenes = view.spores.filter((n) => n.address.startsWith('experience:demo/scene:'));
	assert.deepEqual(scenes.map((n) => n.enter).sort(), ['experience:demo/scene:back', 'experience:demo/scene:poster']);
	assert.ok(view.spores.every((n) => n.driver === 'xr'));
	assert.ok(view.spores.some((n) => n.address === 'experience:demo/plugin:spin'));
	const signals = new Set(view.threads.map((e) => e.signal));
	assert.ok(![...signals].some((s) => s === ('value' as never) || s === ('node' as never) || s === ('media' as never)), 'OSCT signals mapped to the deck table');
});

test('the scene level: the node rule, its home as a JSON pointer into the shared entity, stubs to the other scene', () => {
	const view = projectXr(doc, 'experience:demo/scene:poster');
	assert.equal(view.context, 'experience:demo/scene:poster');
	assert.deepEqual(view.levels.map((l) => l.address), ['experience:demo', 'experience:demo/scene:poster']);
	const homes = view.threads.flatMap((e) => (e.home ? [e.home.path] : []));
	assert.ok(homes.some((p) => p.startsWith('/configuration/entities/overlay/children/0/components/0/rules/0')), JSON.stringify(homes));
	assert.ok(homes.some((p) => p.startsWith('/configuration/scenes/0/logic/rules/0')), JSON.stringify(homes));
	assert.ok(view.threads.every((e) => !e.home || e.home.doc === 'experience:demo'));
	for (const stub of view.stubs) {
		const edge = view.threads.find((e) => e.id === stub.thread);
		assert.ok(edge, `the stub's edge ${stub.thread} is drawn`);
		const far = view.spores.find((n) => n.id === (stub.side === 'out' ? edge.to.spore : edge.from.spore));
		assert.equal(far?.family, 'far');
		assert.equal(far?.address, stub.far);
	}
	assert.ok(view.stubs.some((s) => s.far === 'experience:demo/scene:back'), JSON.stringify(view.stubs));
});

test('every thread joins spores and ports of the weave; an unknown address falls back to the experience', () => {
	for (const at of ['experience:demo', 'experience:demo/scene:poster', 'experience:demo/scene:back', 'experience:demo/scene:poster/target:cat']) {
		const view = projectXr(doc, at);
		const ports = new Map(view.spores.map((n) => [n.id, new Set(n.ports.map((p) => p.id))]));
		for (const e of view.threads) {
			assert.ok(ports.get(e.from.spore)?.has(e.from.port), `${at}: ${e.id} from ${e.from.spore}.${e.from.port}`);
			assert.ok(ports.get(e.to.spore)?.has(e.to.port), `${at}: ${e.id} to ${e.to.spore}.${e.to.port}`);
		}
	}
	assert.equal(projectXr(doc, 'chapter:sandbox/1/2').context, 'experience:demo');
});

test('the xr vector in the contract’s shared vectors is what the projector gives', () => {
	const vectors = JSON.parse(readFileSync(new URL('../../../server/schema/api/spore-network.vectors.json', import.meta.url), 'utf8')) as { weaves: { scope: string; context: string }[] };
	const xr = vectors.weaves.find((v) => v.scope === 'xr');
	assert.ok(xr, 'an xr view in the vectors');
	assert.deepEqual(projectXr(doc, xr.context), xr);
});
