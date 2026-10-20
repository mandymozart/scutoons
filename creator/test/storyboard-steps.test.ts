// The Storyboard page's steps (src/lib/editor/storyboard-steps.ts, #123): each step remembers its panels and the
// inspector's tab; a broken or blocked storage falls back to the step's first layout.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { loadLayout, loadStep, READY_STEPS, saveLayout, saveStep, STEPS, stepKey, stepOfKey, togglePanel, type KeyValue } from '../src/lib/editor/storyboard-steps.ts';

const memory = (): KeyValue & { map: Map<string, string> } => {
	const map = new Map<string, string>();
	return { map, getItem: (k) => map.get(k) ?? null, setItem: (k, v) => void map.set(k, v) };
};
const step = (id: string) => STEPS.find((s) => s.id === id)!;

test('the six steps keep their keys, all ready (Handheld and Export since #126)', () => {
	assert.deepEqual(STEPS.map((s) => s.key), [1, 2, 3, 4, 5, 6]);
	assert.deepEqual(READY_STEPS.map((s) => s.id), ['script', 'shots', 'layout', 'handheld', 'generate', 'export']);
	assert.equal(stepOfKey('5')?.id, 'generate');
	assert.equal(stepOfKey('4')?.id, 'handheld');
	assert.deepEqual([step('handheld').panels, step('export').panels], [['handheld'], ['output']]);
	assert.equal(stepOfKey('x'), null);
});

test('a step opens its own panels the first time, then what it remembers', () => {
	const s = memory();
	assert.deepEqual(loadLayout(s, step('script')), { panels: ['shot', 'script'], tab: 'shot' });
	assert.deepEqual(loadLayout(s, step('layout')), { panels: ['shot'], tab: 'camera' });
	saveLayout(s, 'layout', { panels: ['shot', 'rhythm'], tab: 'light' });
	assert.deepEqual(loadLayout(s, step('layout')), { panels: ['shot', 'rhythm'], tab: 'light' });
	// another step is untouched
	assert.deepEqual(loadLayout(s, step('shots')), { panels: ['shot'], tab: 'shot' });
});

test('a remembered layout loses unknown kinds and doubles; a bad tab falls back', () => {
	const s = memory();
	s.setItem(stepKey('shots'), JSON.stringify({ panels: ['shot', 'luma', 'shot', 'generate'], tab: 'lens' }));
	assert.deepEqual(loadLayout(s, step('shots')), { panels: ['shot', 'generate'], tab: 'shot' });
	s.setItem(stepKey('shots'), '{not json');
	assert.deepEqual(loadLayout(s, step('shots')), { panels: ['shot'], tab: 'shot' });
});

test('a storage that throws never breaks the page', () => {
	const broken: KeyValue = {
		getItem: () => {
			throw new Error('blocked');
		},
		setItem: () => {
			throw new Error('full');
		}
	};
	assert.deepEqual(loadLayout(broken, step('generate')), { panels: ['generate'], tab: 'shot' });
	assert.doesNotThrow(() => saveLayout(broken, 'generate', { panels: [], tab: 'shot' }));
	assert.equal(loadStep(broken), 'shots');
	assert.equal(loadStep(null), 'shots');
});

test('the current step is remembered – an unknown one falls back', () => {
	const s = memory();
	saveStep(s, 'generate');
	assert.equal(loadStep(s), 'generate');
	saveStep(s, 'handheld');
	assert.equal(loadStep(s), 'handheld');
	saveStep(s, 'gone');
	assert.equal(loadStep(s), 'shots');
});

test('a panel toggles in and out of a layout', () => {
	const l = { panels: ['shot' as const], tab: 'shot' as const };
	assert.equal(togglePanel(l, 'shot', true), l);
	assert.deepEqual(togglePanel(l, 'script', true).panels, ['shot', 'script']);
	assert.deepEqual(togglePanel(l, 'shot', false).panels, []);
});
