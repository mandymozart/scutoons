// The record stage's gestures (owner 2026-10-16, the gesture map): one finger – a tap (< 10 px, < 300 ms) acts on what
// is under it (the stage decides: focus the records / the turntable, select a record, turn its sleeve round); a quick
// swipe over the records cycles the carousel (never the camera); press and hold (~280 ms without moving) on the front
// record then drag = carry it – let go over the turntable to put it on; hold anywhere else then drag = the camera ride.
// Two fingers always move the camera – sideways = ride, pinch = zoom; the mouse wheel zooms, a mouse holds and drags
// like a finger. Plain JS (record-stage.js); the types in record-gestures.d.ts.

const HOLD_MS = 280;
const SLOP = 10; // px a finger may wander and still tap / hold

export function stageGestures(canvas, on) {
	const pointers = new Map();
	// one finger: 'wait' (tap / swipe / hold undecided) | 'ride' | 'swipe' | 'carry' | 'none'; two: 'two'
	let mode = 'none';
	let start = null, timer = 0, pinch = 0;
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
		pointers.set(e.pointerId, { x: e.clientX, y: e.clientY, lastX: e.clientX });
		clearTimeout(timer);
		if (pointers.size === 1) {
			const p = local(e);
			start = { x: e.clientX, y: e.clientY, at: performance.now(), hit: on.region(p.x, p.y) };
			mode = 'wait';
			timer = setTimeout(() => {
				if (mode !== 'wait') return;
				if (start.hit.front && on.carry(true)) mode = 'carry';
				else {
					mode = 'ride';
					on.hold();
				}
			}, HOLD_MS);
		} else if (pointers.size === 2) {
			if (mode === 'carry') on.carry(false);
			mode = 'two';
			pinch = spread();
			on.hold();
		}
	};
	const move = (e) => {
		const p = pointers.get(e.pointerId);
		if (!p) return;
		const before = mode === 'two' ? middle() : p.lastX;
		p.x = e.clientX;
		p.y = e.clientY;
		const w = canvas.clientWidth || 300;
		if (mode === 'two') {
			on.ride((middle() - before) / w);
			const now = spread();
			on.zoom((now / pinch - 1) * 0.9);
			pinch = now;
		} else if (mode === 'ride') on.ride((e.clientX - before) / w);
		else if (mode === 'carry') {
			const q = local(e);
			on.carryTo(q.x, q.y);
		} else if (mode === 'wait' && Math.hypot(e.clientX - start.x, e.clientY - start.y) > SLOP) {
			clearTimeout(timer);
			mode = start.hit.part === 'records' ? 'swipe' : 'none';
		}
		p.lastX = e.clientX;
	};
	const up = (e) => {
		if (!pointers.has(e.pointerId)) return;
		pointers.delete(e.pointerId);
		clearTimeout(timer);
		if (pointers.size > 0) return;
		const dx = e.clientX - (start?.x ?? e.clientX), dy = e.clientY - (start?.y ?? e.clientY);
		if (mode === 'wait' && performance.now() - start.at < 300) on.tap(start.hit);
		// up or left = the front record flicks away to the back (the next comes forward); down / right = back again
		else if (mode === 'swipe') on.swipe((Math.abs(dy) > Math.abs(dx) ? dy < 0 : dx < 0) ? 1 : -1);
		else if (mode === 'carry') {
			const q = local(e);
			on.drop(on.region(q.x, q.y).part);
		} else if (mode === 'ride' || mode === 'two') on.release();
		mode = 'none';
	};
	const wheel = (e) => {
		e.preventDefault();
		on.zoom(-e.deltaY * 0.0012);
	};
	canvas.addEventListener('pointerdown', down);
	canvas.addEventListener('pointermove', move);
	canvas.addEventListener('pointerup', up);
	canvas.addEventListener('pointercancel', up);
	canvas.addEventListener('wheel', wheel, { passive: false });
	return () => {
		clearTimeout(timer);
		canvas.removeEventListener('pointerdown', down);
		canvas.removeEventListener('pointermove', move);
		canvas.removeEventListener('pointerup', up);
		canvas.removeEventListener('pointercancel', up);
		canvas.removeEventListener('wheel', wheel);
	};
}
