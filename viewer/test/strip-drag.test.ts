// The endless band's input (components/toon/renderer/strip-drag.ts; BACKLOG 10.100, TEXT-ACROSS-SLICES.md §8):
//   npm test -w viewer
// Measured in the prototype: plain `pan-y` blocked pinch-zoom on the band, a mouse-down on a button in a partly shown
// frame focused it and its slide fought the drag, Swiper trapped the vertical wheel. The attachment runs on a stand-in
// element (an EventTarget with the few fields it reads).
import { mock, test } from 'node:test';
import assert from 'node:assert/strict';
import { gestureAxis, stripDrag, TOUCH_ACTION, WHEEL_BURST, wheelLatch, type StripInput } from '../src/lib/components/toon/renderer/strip-drag.ts';
import type { Swipe } from '../src/lib/components/toon/renderer/strip-loop.ts';

class Band extends EventTarget {
	rem = 16;
	clientWidth = 390;
	captured: number | null = null;
	style = { touchAction: 'auto' };
	ownerDocument = {
		documentElement: { dataset: {} as Record<string, string> },
		defaultView: { getComputedStyle: () => ({ fontSize: `${this.rem}px` }) }
	};
	setPointerCapture(id: number) {
		this.captured = id;
	}
}

function attach(overrides: Partial<StripInput> = {}) {
	const log = { moves: [] as number[], releases: [] as number[], swipes: [] as (Swipe | undefined)[], steps: [] as number[], stops: 0, reveals: [] as unknown[], dragging: [] as boolean[] };
	let moving = false;
	const input: StripInput = {
		enabled: () => true,
		stop: () => {
			log.stops++;
			const was = moving;
			moving = false;
			return was;
		},
		move: (dx) => void log.moves.push(dx),
		release: (v, swipe) => void (log.releases.push(v), log.swipes.push(swipe)),
		step: (d) => void log.steps.push(d),
		reveal: (t) => void log.reveals.push(t),
		dragging: (on) => void log.dragging.push(on),
		...overrides
	};
	const band = new Band();
	const detach = stripDrag(input)(band as unknown as HTMLElement) as () => void;
	return { band, log, detach, glide: () => (moving = true) };
}

/** dispatch a stand-in event with these fields; returns it (defaultPrevented) */
function fire(target: EventTarget, type: string, fields: Record<string, unknown> = {}) {
	const e = new Event(type, { cancelable: true });
	for (const [key, value] of Object.entries(fields)) Object.defineProperty(e, key, { value });
	target.dispatchEvent(e);
	return e;
}
const pointer = (band: Band, type: string, x: number, y: number, t: number, more: Record<string, unknown> = {}) =>
	fire(band, type, { pointerId: 1, pointerType: 'touch', button: 0, buttons: 1, clientX: x, clientY: y, timeStamp: t, ...more });
const wheel = (band: Band, dx: number, dy: number, t: number, more: Record<string, unknown> = {}) =>
	fire(band, 'wheel', { deltaX: dx, deltaY: dy, deltaMode: 0, shiftKey: false, ctrlKey: false, timeStamp: t, ...more });

test('touch-action pan-y pinch-zoom while attached (plain pan-y blocks zooming on the band), the old value after', () => {
	const { band, detach } = attach();
	assert.equal(band.style.touchAction, TOUCH_ACTION);
	assert.equal(TOUCH_ACTION, 'pan-y pinch-zoom');
	detach();
	assert.equal(band.style.touchAction, 'auto');
});

test('the dead zone: 0.5rem, then the axis locks and the band follows 1:1 from the touch point', () => {
	assert.equal(gestureAxis(7.9, 0, 8), null);
	assert.equal(gestureAxis(8.1, 0, 8), 'x');
	assert.equal(gestureAxis(-6, 6, 8), 'y', 'a tie goes to the page');
	assert.equal(gestureAxis(2, -8.5, 8), 'y');
	const { band, log } = attach();
	pointer(band, 'pointerdown', 100, 100, 0);
	pointer(band, 'pointermove', 107, 101, 10);
	assert.deepEqual(log.moves, []);
	assert.ok(pointer(band, 'pointermove', 109, 101, 20).defaultPrevented);
	pointer(band, 'pointermove', 120, 103, 30);
	assert.deepEqual(log.moves, [9, 11]);
	assert.deepEqual(log.dragging, [true]);
	assert.equal(band.captured, 1);
	// the capture moving from the touched slice to the band: the slice's lostpointercapture bubbles up – the drag goes on
	fire(band, 'lostpointercapture', { pointerId: 1, target: {} });
	pointer(band, 'pointermove', 130, 103, 40);
	assert.deepEqual(log.moves, [9, 11, 10]);
	assert.deepEqual(log.releases, []);
	// the band's own capture lost: the drag ends
	fire(band, 'lostpointercapture', { pointerId: 1, timeStamp: 50 });
	assert.equal(log.releases.length, 1);
	// a larger root font: a larger zone
	const big = attach();
	big.band.rem = 20;
	pointer(big.band, 'pointerdown', 100, 100, 0);
	pointer(big.band, 'pointermove', 109, 100, 10);
	assert.deepEqual(big.log.moves, []);
});

test('a second finger during a drag (pinch-zoom): the band goes back to where the drag began, no fling', () => {
	const { band, log } = attach();
	pointer(band, 'pointerdown', 100, 100, 0);
	pointer(band, 'pointermove', 112, 100, 10);
	pointer(band, 'pointermove', 116, 100, 20);
	pointer(band, 'pointerdown', 200, 300, 25, { pointerId: 2 });
	assert.deepEqual(log.moves, [12, 4, -16]);
	assert.deepEqual(log.releases, [0]);
	assert.deepEqual(log.dragging, [true, false]);
	// the fingers' further moves are the browser's zoom
	pointer(band, 'pointermove', 140, 100, 30);
	pointer(band, 'pointermove', 240, 300, 30, { pointerId: 2 });
	assert.deepEqual(log.moves, [12, 4, -16]);
});

test('a gesture that leaves the dead zone up / down is the page\'s for its whole way', () => {
	const { band, log } = attach();
	pointer(band, 'pointerdown', 100, 100, 0);
	assert.equal(pointer(band, 'pointermove', 103, 112, 10).defaultPrevented, false);
	pointer(band, 'pointermove', 160, 120, 20);
	pointer(band, 'pointerup', 160, 120, 30);
	assert.deepEqual(log.moves, []);
	assert.deepEqual(log.releases, []);
	assert.equal(band.captured, null);
	assert.equal(fire(band, 'click', { detail: 1 }).defaultPrevented, false);
});

test('the click after a drag is swallowed, a keyboard click passes, a plain tap is a click', () => {
	const { band } = attach();
	pointer(band, 'pointerdown', 100, 100, 0);
	pointer(band, 'pointermove', 140, 100, 16);
	pointer(band, 'pointerup', 140, 100, 24);
	assert.equal(fire(band, 'click', { detail: 0 }).defaultPrevented, false, 'Enter on a focused button');
	assert.equal(fire(band, 'click', { detail: 1 }).defaultPrevented, true);
	assert.equal(fire(band, 'click', { detail: 1 }).defaultPrevented, false, 'only the one click');
	pointer(band, 'pointerdown', 100, 100, 100);
	pointer(band, 'pointerup', 101, 100, 180);
	assert.equal(fire(band, 'click', { detail: 1 }).defaultPrevented, false);
	// a drag that ended off any button leaves no swallow for the next tap
	pointer(band, 'pointerdown', 100, 100, 200);
	pointer(band, 'pointermove', 160, 100, 216);
	pointer(band, 'pointerup', 160, 100, 224);
	pointer(band, 'pointerdown', 100, 100, 400);
	pointer(band, 'pointerup', 100, 100, 450);
	assert.equal(fire(band, 'click', { detail: 1 }).defaultPrevented, false);
});

test('a tap that stops a running glide is no click; any new drag / touch stops the motion first', () => {
	const { band, log, glide } = attach();
	glide();
	pointer(band, 'pointerdown', 100, 100, 0);
	pointer(band, 'pointerup', 100, 100, 60);
	assert.equal(log.stops, 1);
	assert.equal(fire(band, 'click', { detail: 1 }).defaultPrevented, true);
	assert.deepEqual(log.releases, [0], 'the stopped motion comes to rest (a pager settles on a page)');
	assert.deepEqual(log.swipes, [undefined]);
	pointer(band, 'pointerdown', 100, 100, 100);
	assert.equal(log.stops, 2);
	pointer(band, 'pointerup', 100, 100, 150);
	assert.deepEqual(log.releases, [0], 'a plain tap releases nothing');
	// a press that stopped a motion, then a vertical swipe (the page's): at rest too
	glide();
	pointer(band, 'pointerdown', 100, 100, 200);
	pointer(band, 'pointermove', 101, 120, 216);
	assert.deepEqual(log.releases, [0, 0]);
});

test('a drag\'s release carries the whole swipe (way, time) for a pager\'s short-swipe rule; a cancel or a pinch none', () => {
	const { band, log } = attach();
	pointer(band, 'pointerdown', 100, 100, 1000);
	pointer(band, 'pointermove', 80, 100, 1016);
	pointer(band, 'pointermove', 60, 100, 1032);
	pointer(band, 'pointerup', 60, 100, 1040);
	assert.deepEqual(log.swipes, [{ dx: -40, ms: 40 }]);
	pointer(band, 'pointerdown', 100, 100, 2000);
	pointer(band, 'pointermove', 140, 100, 2016);
	pointer(band, 'pointercancel', 140, 100, 2030);
	pointer(band, 'pointerdown', 100, 100, 3000);
	pointer(band, 'pointermove', 140, 100, 3016);
	pointer(band, 'pointerdown', 300, 100, 3020, { pointerId: 2 });
	assert.deepEqual(log.swipes.slice(1), [undefined, undefined]);
	assert.deepEqual(log.releases.slice(1), [0, 0]);
});

test('a sideways wheel burst settles once, 150 ms after its last event (only with a settle hook)', () => {
	mock.timers.enable({ apis: ['setTimeout'] });
	try {
		let settled = 0;
		const { band, detach } = attach({ settle: () => void settled++ });
		wheel(band, 30, 0, 0);
		mock.timers.tick(WHEEL_BURST - 10);
		wheel(band, 30, 0, 140);
		mock.timers.tick(WHEEL_BURST - 10);
		assert.equal(settled, 0, 'the burst goes on');
		mock.timers.tick(20);
		assert.equal(settled, 1);
		wheel(band, 0, 100, 1000);
		mock.timers.tick(WHEEL_BURST * 2);
		assert.equal(settled, 1, 'the page\'s wheel settles nothing');
		wheel(band, 30, 0, 2000);
		detach();
		mock.timers.tick(WHEEL_BURST * 2);
		assert.equal(settled, 1, 'detached: no settle');
		const plain = attach();
		assert.ok(wheel(plain.band, 30, 0, 0).defaultPrevented, 'no hook: the wheel still moves it');
	} finally {
		mock.timers.reset();
	}
});

test('the release velocity: the last 100 ms of the drag; a rest before letting go or a cancel is no fling', () => {
	const drag = (band: Band, upAt: number, type = 'pointerup') => {
		pointer(band, 'pointerdown', 100, 100, 0);
		for (let t = 16; t <= 160; t += 16) pointer(band, 'pointermove', 100 + 2 * t, 100, t);
		pointer(band, type, 420, 100, upAt);
	};
	const fling = attach();
	drag(fling.band, 168);
	assert.equal(fling.log.releases.length, 1);
	assert.ok(Math.abs(fling.log.releases[0] - 2) < 1e-9, `${fling.log.releases[0]}`);
	assert.deepEqual(fling.log.dragging, [true, false]);
	const rested = attach();
	drag(rested.band, 240);
	assert.deepEqual(rested.log.releases, [0]);
	const cancelled = attach();
	drag(cancelled.band, 168, 'pointercancel');
	assert.deepEqual(cancelled.log.releases, [0]);
});

test('the wheel: deltaX-led or Shift only, the axis latched per 150 ms burst, vertical is the page\'s', () => {
	const latch = wheelLatch();
	const e = (deltaX: number, deltaY: number, timeStamp: number, shiftKey = false) => ({ deltaX, deltaY, timeStamp, shiftKey });
	assert.equal(latch(e(0, 100, 0)), null);
	assert.equal(latch(e(30, 2, 100)), null, 'still the page\'s burst');
	assert.equal(latch(e(30, 2, 240)), null, 'events 140 ms apart: one burst');
	assert.equal(latch(e(30, 0, 400)), 30, 'a new burst');
	assert.equal(latch(e(0, 40, 450)), 0, 'the band\'s burst – the page doesn\'t scroll in it');
	assert.equal(latch(e(0, 100, 1000, true)), 100, 'Shift turns a vertical wheel sideways');
	const { band, log } = attach();
	const sideways = wheel(band, 50, 3, 0);
	assert.ok(sideways.defaultPrevented);
	assert.deepEqual(log.moves, [-50]);
	assert.equal(log.stops, 1, 'a wheel stops a running glide');
	assert.equal(wheel(band, 0, 100, 1000).defaultPrevented, false);
	assert.equal(log.stops, 1, 'the page\'s wheel leaves the band alone');
	wheel(band, 3, 0, 2000, { deltaMode: 1 });
	assert.deepEqual(log.moves, [-50, -48], 'lines are rem');
	assert.equal(wheel(band, 80, 0, 3000, { ctrlKey: true }).defaultPrevented, false, 'a trackpad pinch zooms');
});

test('← → step while the band has focus – not from a text field, not with a modifier', () => {
	const { band, log } = attach();
	assert.ok(fire(band, 'keydown', { key: 'ArrowRight' }).defaultPrevented);
	fire(band, 'keydown', { key: 'ArrowLeft' });
	assert.deepEqual(log.steps, [1, -1]);
	fire(band, 'keydown', { key: 'ArrowDown' });
	fire(band, 'keydown', { key: 'ArrowRight', altKey: true });
	assert.equal(fire(band, 'keydown', { key: 'ArrowRight', target: { tagName: 'INPUT' } }).defaultPrevented, false);
	fire(band, 'keydown', { key: 'ArrowLeft', target: { tagName: 'DIV', isContentEditable: true } });
	assert.deepEqual(log.steps, [1, -1]);
});

test('keyboard focus slides its frame in; a pointer\'s focus doesn\'t (it fought the drag)', () => {
	const { band, log } = attach();
	const button = (visible: boolean) => ({ matches: (s: string) => visible && s === ':focus-visible' });
	const keyboard = button(true);
	fire(band, 'focusin', { target: keyboard });
	assert.deepEqual(log.reveals, [keyboard]);
	fire(band, 'focusin', { target: button(false) });
	pointer(band, 'pointerdown', 100, 100, 0);
	fire(band, 'focusin', { target: keyboard });
	pointer(band, 'pointerup', 100, 100, 50);
	fire(band, 'focusin', { target: band });
	assert.equal(log.reveals.length, 1);
});

test('no input while blocked (the band: the story blocks the page) or still', () => {
	let story = true;
	const { band, log } = attach({ blocked: () => story });
	pointer(band, 'pointerdown', 100, 100, 0);
	pointer(band, 'pointermove', 160, 100, 16);
	assert.equal(wheel(band, 50, 0, 0).defaultPrevented, false);
	fire(band, 'keydown', { key: 'ArrowRight' });
	assert.deepEqual([log.moves, log.steps, log.stops], [[], [], 0]);
	// the story takes over mid-drag: the drag ends without a fling
	story = false;
	pointer(band, 'pointerdown', 100, 100, 100);
	pointer(band, 'pointermove', 140, 100, 116);
	story = true;
	pointer(band, 'pointermove', 180, 100, 132);
	assert.deepEqual(log.moves, [40]);
	assert.deepEqual(log.releases, [0]);
	// the utility reads no page state of its own: without `blocked` the html flag is the band's business
	const free = attach();
	free.band.ownerDocument.documentElement.dataset.vnBlocking = '';
	pointer(free.band, 'pointerdown', 100, 100, 0);
	pointer(free.band, 'pointermove', 160, 100, 16);
	assert.deepEqual(free.log.moves, [60]);
	const still = attach({ enabled: () => false });
	pointer(still.band, 'pointerdown', 100, 100, 0);
	pointer(still.band, 'pointermove', 160, 100, 16);
	assert.equal(wheel(still.band, 50, 0, 0).defaultPrevented, false);
	assert.equal(fire(still.band, 'keydown', { key: 'ArrowRight' }).defaultPrevented, false);
	assert.deepEqual([still.log.moves, still.log.steps], [[], []]);
});

test('a mouse: the main button only; a release missed outside the window ends the drag', () => {
	const { band, log } = attach();
	pointer(band, 'pointerdown', 100, 100, 0, { pointerType: 'mouse', button: 2 });
	pointer(band, 'pointermove', 160, 100, 16, { pointerType: 'mouse', buttons: 2 });
	assert.deepEqual(log.moves, []);
	pointer(band, 'pointerdown', 100, 100, 100, { pointerType: 'mouse' });
	pointer(band, 'pointermove', 140, 100, 116, { pointerType: 'mouse' });
	pointer(band, 'pointermove', 150, 100, 132, { pointerType: 'mouse', buttons: 0 });
	assert.deepEqual(log.moves, [40]);
	assert.equal(log.releases.length, 1);
	assert.deepEqual(log.dragging, [true, false]);
	// a second finger during a drag is ignored
	pointer(band, 'pointerdown', 100, 100, 200);
	pointer(band, 'pointerdown', 300, 100, 210, { pointerId: 2 });
	pointer(band, 'pointermove', 340, 100, 220, { pointerId: 2 });
	assert.deepEqual(log.moves, [40]);
});
