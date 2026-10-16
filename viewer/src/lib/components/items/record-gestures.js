// The record stage's gestures (owner 2026-10-16, the gesture map): one finger – a tap jumps the camera to the section
// under it (the records / the turntable), a swipe over the records cycles the carousel (never the camera), press and
// hold (~350 ms without moving) then drag rides the camera; two fingers always move the camera – sideways = ride,
// pinch = zoom; the mouse wheel zooms, a mouse holds and drags like a finger. Plain JS (record-stage.js).

const HOLD_MS = 350;
const SLOP = 8; // px a finger may wander and still tap / hold

/**
 * @param {HTMLCanvasElement} canvas
 * @param {{ region: (x: number, y: number) => 'records' | 'deck', tap: (where: 'records' | 'deck') => void,
 *   swipe: (step: 1 | -1) => void, ride: (dx: number) => void, zoom: (by: number) => void, hold: () => void,
 *   release: () => void }} on  dx in canvas widths; zoom: a change of the 0…1 zoom
 * @returns {() => void} removes the listeners
 */
export function stageGestures(canvas, on) {
	const pointers = new Map();
	let mode = 'none'; // one finger: 'wait' (tap / swipe / hold undecided) | 'ride' | 'swipe' | 'none'; two: 'two'
	let start = null, timer = 0, pinch = 0, wheelTimer = 0;
	const local = (e) => {
		const r = canvas.getBoundingClientRect();
		return { x: e.clientX - r.left, y: e.clientY - r.top };
	};
	const spread = () => {
		const [a, b] = [...pointers.values()];
		return Math.hypot(a.x - b.x, a.y - b.y) || 1;
	};
	const middle = () => [...pointers.values()].reduce((s, p) => s + p.x, 0) / pointers.size;

	const down = (e) => {
		try {
			canvas.setPointerCapture?.(e.pointerId);
		} catch {
			/* a pointer the browser doesn't track (synthetic): still follow it */
		}
		pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
		clearTimeout(timer);
		if (pointers.size === 1) {
			const p = local(e);
			start = { x: e.clientX, y: e.clientY, at: performance.now(), where: on.region(p.x, p.y) };
			mode = 'wait';
			timer = setTimeout(() => {
				if (mode !== 'wait') return;
				mode = 'ride';
				on.hold();
			}, HOLD_MS);
		} else if (pointers.size === 2) {
			mode = 'two';
			pinch = spread();
			on.hold();
		}
	};
	const move = (e) => {
		const p = pointers.get(e.pointerId);
		if (!p) return;
		const before = mode === 'two' ? middle() : p.x;
		p.x = e.clientX;
		p.y = e.clientY;
		const w = canvas.clientWidth || 300;
		if (mode === 'two') {
			on.ride((middle() - before) / w);
			const now = spread();
			on.zoom((now / pinch - 1) * 0.9);
			pinch = now;
		} else if (mode === 'ride') on.ride((e.clientX - (p.lastX ?? start.x)) / w);
		else if (mode === 'wait' && Math.hypot(e.clientX - start.x, e.clientY - start.y) > SLOP) {
			clearTimeout(timer);
			mode = start.where === 'records' ? 'swipe' : 'none';
		}
		p.lastX = e.clientX;
	};
	const up = (e) => {
		if (!pointers.has(e.pointerId)) return;
		pointers.delete(e.pointerId);
		clearTimeout(timer);
		if (pointers.size > 0) return;
		const dx = e.clientX - (start?.x ?? e.clientX), dy = e.clientY - (start?.y ?? e.clientY);
		if (mode === 'wait') on.tap(start.where);
		// up or left = the front record flicks away to the back (the next comes forward); down / right = back again
		else if (mode === 'swipe') on.swipe((Math.abs(dy) > Math.abs(dx) ? dy < 0 : dx < 0) ? 1 : -1);
		if (mode === 'ride' || mode === 'two') on.release();
		mode = 'none';
	};
	const wheel = (e) => {
		e.preventDefault();
		on.hold();
		on.zoom(-e.deltaY * 0.0012);
		clearTimeout(wheelTimer);
		wheelTimer = setTimeout(on.release, 1400);
	};
	canvas.addEventListener('pointerdown', down);
	canvas.addEventListener('pointermove', move);
	canvas.addEventListener('pointerup', up);
	canvas.addEventListener('pointercancel', up);
	canvas.addEventListener('wheel', wheel, { passive: false });
	return () => {
		clearTimeout(timer);
		clearTimeout(wheelTimer);
		canvas.removeEventListener('pointerdown', down);
		canvas.removeEventListener('pointermove', move);
		canvas.removeEventListener('pointerup', up);
		canvas.removeEventListener('pointercancel', up);
		canvas.removeEventListener('wheel', wheel);
	};
}
