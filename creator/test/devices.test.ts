// Resume across devices (lib/shell/devices.ts): the state read back, which start the server's rows give, the panels fitted
// into another window, the device's id and name.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { deviceName, fitPanels, fitPlace, hostPanels, isBlank, newDeviceId, NO_DEVICE_STATE, readDeviceState, readMeta, resumption, seenKey, whenLabel, withHostPanels, type DeviceRow, type DeviceState } from '../src/lib/shell/devices.ts';
import { NO_SESSION } from '../src/lib/shell/resume.ts';

const tabs = { tabs: [{ address: 'toon:catpope', href: '/catpope', label: 'Cat Pope' }], front: 'toon:catpope' };
const studio: DeviceState = {
	v: 1,
	window: { w: 2000, h: 1200 },
	tabs,
	chosen: { character: 'own-abc' },
	panels: { 'creator-host-character': { places: { left: { corner: 'top-right', dx: 1000, dy: 100, size: { w: 600, h: 400 } } }, stage: { layout: 'one', slots: ['moods'] } } },
	views: { character: { split: 0.4 } },
	session: { dashboard: false, full: { 'creator-host-character': 'moods' }, scroll: { 'stage:toon:catpope': { top: 300, path: [0] } } }
};
const row = (device: string, at: string, state: unknown = studio): DeviceRow => ({ device, name: 'Firefox · Windows', state, updated_at: at });
const blank: DeviceState = { ...NO_DEVICE_STATE, window: { w: 1280, h: 800 } };

test('a state reads back whole; another version or a broken one reads as null', () => {
	assert.deepEqual(readDeviceState(JSON.parse(JSON.stringify(studio))), studio);
	assert.equal(readDeviceState(null), null);
	assert.equal(readDeviceState({ ...studio, v: 2 }), null);
	assert.equal(readDeviceState({ ...studio, window: { w: 0, h: 5 } }), null);
	const odd = readDeviceState({ ...studio, panels: { 'kit-a': {}, 'creator-host-x': 'no' }, session: 'x', chosen: [1] });
	assert.deepEqual([odd?.panels, odd?.session, odd?.chosen], [{}, NO_SESSION, {}]);
});

test('only the creator’s workspaces leave the kit’s store, and only they are replaced', () => {
	const stored = { 'kit-scene': { places: {} }, 'creator-host-toon': { places: {} }, 'creator-host-bad': 3 };
	assert.deepEqual(hostPanels(stored), { 'creator-host-toon': { places: {} } });
	assert.deepEqual(withHostPanels(stored, { 'creator-host-character': { stage: {} } }), { 'kit-scene': { places: {} }, 'creator-host-character': { stage: {} } });
});

test('another device’s newer state is offered once; this device’s own counts first', () => {
	const other = row('laptop-0002', '2026-11-24T09:02:00Z');
	const offer = resumption(studio, Date.parse('2026-11-23T23:14:00Z'), row('studio-0001', '2026-11-23T23:14:00Z'), other, null);
	assert.equal(offer.kind, 'offer');
	// offered already, older than this device's last change (local or on the server), or nothing in it: not offered
	assert.equal(resumption(studio, 0, null, other, seenKey(other)).kind, 'none');
	assert.equal(resumption(studio, Date.parse('2026-11-24T10:00:00Z'), null, other, null).kind, 'none');
	assert.equal(resumption(studio, 0, row('studio-0001', '2026-11-24T10:00:00Z'), other, null).kind, 'none');
	assert.equal(resumption(studio, 0, null, row('laptop-0002', '2026-11-24T09:02:00Z', blank), null).kind, 'none');
	assert.equal(resumption(studio, 0, null, row('laptop-0002', '2026-11-24T09:02:00Z', { v: 9 }), null).kind, 'none');
});

test('the same device restores silently only what this browser lost', () => {
	const mine = row('studio-0001', '2026-11-24T09:00:00Z');
	const own = resumption(blank, 0, mine, null, null);
	assert.equal(own.kind, 'own');
	assert.deepEqual(own.kind === 'own' && own.state.tabs, tabs);
	assert.equal(resumption(studio, 0, mine, null, null).kind, 'none');
	assert.equal(resumption(blank, 0, row('studio-0001', '2026-11-24T09:00:00Z', blank), null, null).kind, 'none');
	assert.equal(isBlank(blank), true);
});

test('panels placed on a larger window stay inside a smaller one', () => {
	const place = fitPlace({ corner: 'top-right', dx: 1000, dy: 100, size: { w: 600, h: 400 } }, { w: 2000, h: 1200 }, { w: 1000, h: 300 });
	assert.deepEqual(place, { corner: 'top-right', dx: 400, dy: 0, size: { w: 600, h: 300 } });
	const wide = fitPlace({ corner: 'top-left', dx: 1900, dy: 20, size: { w: 1500, h: 100 } }, { w: 2000, h: 1200 }, { w: 1200, h: 1200 });
	assert.deepEqual(wide, { corner: 'top-left', dx: 0, dy: 20, size: { w: 1200, h: 100 } });
	// without a size only its distance scales; the same window changes nothing
	assert.deepEqual(fitPlace({ corner: 'bottom-left', dx: 400, dy: 60 }, { w: 2000, h: 1200 }, { w: 1000, h: 600 }), { corner: 'bottom-left', dx: 200, dy: 30 });
	assert.equal(fitPanels(studio.panels, studio.window, { ...studio.window }), studio.panels);
	const fitted = fitPanels(studio.panels, studio.window, { w: 1000, h: 600 });
	assert.deepEqual((fitted['creator-host-character'].places as Record<string, unknown>).left, { corner: 'top-right', dx: 400, dy: 50, size: { w: 600, h: 400 } });
	assert.deepEqual(fitted['creator-host-character'].stage, studio.panels['creator-host-character'].stage);
});

test('a device keeps its id; a missing or broken one gets a new one', () => {
	const id = newDeviceId((n) => new Uint8Array(n).map((_, i) => i * 7));
	assert.match(id, /^[A-Za-z0-9_-]{22}$/);
	assert.deepEqual(readMeta({ id: 'studio-0001', seen: 'x@1' }, () => 'fresh-000000'), { id: 'studio-0001', seen: 'x@1' });
	assert.deepEqual(readMeta({ id: 'bad id' }, () => 'fresh-000000'), { id: 'fresh-000000', seen: null });
	assert.deepEqual(readMeta(null, () => 'fresh-000000'), { id: 'fresh-000000', seen: null });
});

test('a device is named by its browser and system', () => {
	assert.equal(deviceName('Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:132.0) Gecko/20100101 Firefox/132.0'), 'Firefox · Windows');
	assert.equal(deviceName('Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Safari/605.1.15'), 'Safari · macOS');
	assert.equal(deviceName('Mozilla/5.0 (Windows NT 10.0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0 Safari/537.36 Edg/131.0'), 'Edge · Windows');
	assert.equal(deviceName('Mozilla/5.0 (iPad; CPU OS 17_5 like Mac OS X) AppleWebKit/605.1.15 Version/17.5 Mobile/15E148 Safari/604.1'), 'Safari · iOS');
	assert.equal(deviceName(''), 'Browser');
});

test('the time today, else with the day', () => {
	const now = new Date('2026-11-24T12:00:00');
	assert.match(whenLabel(new Date('2026-11-24T09:02:00').toISOString(), now, 'en'), /09:02/);
	assert.match(whenLabel(new Date('2026-11-23T23:14:00').toISOString(), now, 'en'), /Mon/);
	assert.equal(whenLabel('nonsense', now, 'en'), '');
});
