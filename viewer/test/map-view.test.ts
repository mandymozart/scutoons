// Where the reader's world map opens (src/lib/components/toon/map/map-view.ts, owner 2026-10-10):
//   npm test -w viewer
// first opening: centred on where the reader is at the zoom level; then the remembered view; an explicit focus or a new
// place reached overrides it once.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { HERE_ZOOM, hereZoom, openingView, parseStoredView, readerHere, type OpeningView, type StoredMapView } from '../src/lib/components/toon/map/map-view.ts';

const place = (id: string, x: number, y: number, extra: Record<string, unknown> = {}) =>
	({ id, name: id, x, y, toons: ['catpope'], hidden: true, ...extra }) as any;
const places = [
	place('apartment', 1206, 304, { unlock: { toon: 'catpope', chapter: 1, at: 0.03 } }),
	place('apartment-blocks', 1194, 290, { unlock: { toon: 'catpope', chapter: 1, at: 0.77 } }),
	place('waterfall-tower', 1240, 272, { unlock: { toon: 'catpope', chapter: 2, at: 0.14 } }),
	place('pond', 500, 500, { toons: ['toptoy'] })
];
const state = (unlocked: Record<string, number>, positions: Record<string, unknown> = {}) =>
	({ places: Object.fromEntries(Object.entries(unlocked).map(([id, t]) => [id, { unlocked: t }])), routes: {}, positions }) as any;

test('here: the most recently unlocked place of this toon; ties by story order', () => {
	assert.equal(readerHere(places, 'catpope', state({})), null);
	assert.equal(readerHere(places, 'catpope', state({ apartment: 1 }))?.id, 'apartment');
	// unlocked together (the reading progress): the later one in the story
	assert.equal(readerHere(places, 'catpope', state({ apartment: 5, 'apartment-blocks': 5, 'waterfall-tower': 5 }))?.id, 'waterfall-tower');
	// a later unlock wins over story order (an event, a slice seen later)
	assert.equal(readerHere(places, 'catpope', state({ apartment: 9, 'waterfall-tower': 5 }))?.id, 'apartment');
	// other toons' places are not this toon's "here"
	assert.equal(readerHere(places, 'catpope', state({ pond: 99, apartment: 1 }))?.id, 'apartment');
	assert.deepEqual(readerHere(places, 'catpope', state({ apartment: 1 })), { id: 'apartment', x: 1206, y: 304 });
});

test('here: the story position wins when it is a place on the map', () => {
	const s = state({ apartment: 9 }, { catpope: { location: 'waterfall-tower', at: 1 } });
	assert.equal(readerHere(places, 'catpope', s)?.id, 'waterfall-tower');
	const unknown = state({ apartment: 9 }, { catpope: { location: 'nowhere', at: 1 } });
	assert.equal(readerHere(places, 'catpope', unknown)?.id, 'apartment');
	const onRoute = state({}, { catpope: { route: 'season1', waypoint: 'apartment-blocks', at: 1 } });
	assert.equal(readerHere(places, 'catpope', onRoute)?.id, 'apartment-blocks');
});

test('the zoom level: the toon start view’s zoom, else HERE_ZOOM', () => {
	assert.equal(hereZoom({ center: { x: 0, y: 0 }, zoom: 1.5 }), 1.5);
	assert.equal(hereZoom(undefined), HERE_ZOOM);
	assert.equal(hereZoom({ center: { x: 0, y: 0 }, zoom: 0 }), HERE_ZOOM);
});

/** a view that is not the start view */
const placed = (v: OpeningView) => {
	if (v.kind === 'start') throw new Error('the start view');
	return v;
};

const apartment = { id: 'apartment', x: 1206, y: 304 };
const tower = { id: 'waterfall-tower', x: 1240, y: 272 };

test('first opening: centred on here at the zoom level (and remembered with that here)', () => {
	const v = openingView(null, apartment, 2);
	assert.equal(v.kind, 'here');
	assert.deepEqual([placed(v).center, placed(v).zoom, placed(v).store], [{ x: 1206, y: 304 }, 2, { x: 1206, y: 304, zoom: 2, here: 'apartment' }]);
	// nothing reached and nothing remembered: the toon's start view
	assert.deepEqual(openingView(null, null, 2), { kind: 'start', store: null });
});

test('then: the view the reader left it at', () => {
	const left: StoredMapView = { x: 800, y: 600, zoom: 0.7, here: 'apartment' };
	const v = openingView(left, apartment, 2);
	assert.equal(v.kind, 'stored');
	assert.deepEqual([placed(v).center, placed(v).zoom], [{ x: 800, y: 600 }, 0.7]);
	// moved before anything was reached: remembered too
	assert.equal(openingView({ x: 1, y: 2, zoom: 3, here: null }, null, 2).kind, 'stored');
});

test('a new place reached: centred on it once, then the reader’s view again', () => {
	const left: StoredMapView = { x: 800, y: 600, zoom: 0.7, here: 'apartment' };
	const v = openingView(left, tower, 2);
	assert.equal(v.kind, 'here');
	assert.deepEqual(placed(v).center, { x: 1240, y: 272 });
	// the first place ever reached after moving the map
	assert.equal(openingView({ x: 1, y: 2, zoom: 3, here: null }, apartment, 2).kind, 'here');
	// stored with the new here → the next opening keeps the (then moved) view
	const after = { ...placed(v).store, x: 10, y: 20, zoom: 4 };
	assert.equal(openingView(after, tower, 2).kind, 'stored');
});

test('an explicit focus wins once (even over a new place) and becomes the remembered view', () => {
	const focused: StoredMapView = { x: 300, y: 400, zoom: 0.25, here: 'apartment', focus: true };
	const v = openingView(focused, tower, 2);
	assert.equal(v.kind, 'focus');
	assert.deepEqual([placed(v).center, placed(v).zoom, placed(v).store], [{ x: 300, y: 400 }, 0.25, { x: 300, y: 400, zoom: 0.25, here: 'waterfall-tower' }]);
	const next = placed(v).store;
	assert.equal(openingView(next, tower, 2).kind, 'stored');
});

test('stored values are checked', () => {
	assert.equal(parseStoredView(null), null);
	assert.equal(parseStoredView({ x: 1, y: 2 }), null);
	assert.equal(parseStoredView({ x: 1, y: 2, zoom: 0 }), null);
	assert.equal(parseStoredView({ x: '1', y: 2, zoom: 1 }), null);
	assert.deepEqual(parseStoredView({ x: 1, y: 2, zoom: 1, here: 'a', focus: true, junk: 1 }), { x: 1, y: 2, zoom: 1, here: 'a', focus: true });
	assert.deepEqual(parseStoredView({ x: 1, y: 2, zoom: 1, here: null, focus: 'yes' }), { x: 1, y: 2, zoom: 1 });
});
