import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createExclusive } from '../src/lib/components/panel/exclusive.ts';

/** a panel on a shared state, like the inventory and the avatar on `nav`'s one overlay */
function shared() {
	const state = { overlay: null as string | null };
	const panel = (name: string) => ({ isOpen: () => state.overlay === name, close: () => (state.overlay = null) });
	return { state, panel };
}

test('opening a panel closes the others that are open', () => {
	const exclusive = createExclusive();
	let card = true;
	const removeCard = exclusive.opened({ isOpen: () => card, close: () => (card = false) });
	const { state, panel } = shared();
	state.overlay = 'inventory';
	exclusive.opened(panel('inventory'));
	assert.equal(card, false);
	assert.equal(state.overlay, 'inventory');
	removeCard(); // its cleanup, later: harmless
	assert.equal(state.overlay, 'inventory');
});

test('switching between two panels on one state keeps the new one open', () => {
	const exclusive = createExclusive();
	const { state, panel } = shared();
	state.overlay = 'inventory';
	const removeInventory = exclusive.opened(panel('inventory'));
	// the avatar's top button: the state switches – the inventory is closed already, its entry still registered
	state.overlay = 'avatar';
	exclusive.opened(panel('avatar'));
	assert.equal(state.overlay, 'avatar');
	removeInventory();
	assert.equal(state.overlay, 'avatar');
});

test('a panel that closed and opened again is one entry, not two', () => {
	const exclusive = createExclusive();
	let a = true;
	let closes = 0;
	const pa = { isOpen: () => a, close: () => ((a = false), closes++) };
	exclusive.opened(pa)();
	exclusive.opened(pa);
	let b = true;
	exclusive.opened({ isOpen: () => b, close: () => (b = false) });
	assert.equal(closes, 1);
	assert.equal(a, false);
	assert.equal(b, true);
});
