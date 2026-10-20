// The storyboard's 3D view math (src/lib/editor/storyboard-3d-place.ts, storyboard-3d.ts, #124): the figure placing and
// its exact inverse (a rebuild leaves a moved figure where it was), the hand camera, the orbit's centre, the frame's
// fit, the figure gestures.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Group, OrthographicCamera, PerspectiveCamera, Quaternion, Vector3 } from 'three';
import { applyPlace, D, placeFigure, screenPlace, turnOf, wrapDeg, yawOf, yawToward, type FigurePlace, type Frame } from '../src/lib/editor/storyboard-3d-place.ts';
import { cameraKey, dragTurn, fitFrame, handCamera, moveOnGround, orbitCentre, rem, scaleAbout, turnAbout, wheelFactor, wheelTurn } from '../src/lib/editor/storyboard-3d.ts';

const f: Frame = { W: 390, H: 694, K: 1 };
/** core.js shotCamera's eye-level preset camera (medium: 3.5 m back, 1.55 m up) */
function eyeCamera(): PerspectiveCamera {
	const c = new PerspectiveCamera(50, f.W / f.H, 0.05, 600);
	c.position.set(0, 1.55, 3.5);
	c.updateMatrixWorld();
	return c;
}
function highCamera(): PerspectiveCamera {
	const c = new PerspectiveCamera(50, f.W / f.H, 0.05, 600);
	c.position.set(0, 5, 3.5);
	c.rotation.order = 'YXZ';
	c.rotation.set(-24 * D, 0, 0);
	c.updateMatrixWorld();
	return c;
}
const near = (a: number, b: number, eps = 1e-6) => assert.ok(Math.abs(a - b) < eps, `${a} ≉ ${b}`);

test('a standing figure goes on the ground where the ray through its feet meets it', () => {
	const cam = highCamera();
	const p = placeFigure(cam, f, { x: 0.5, y: 0.6, h: 0.3 }, 1.8, false);
	assert.equal(p.grounded, true);
	near(p.pos.y, 0);
	// the feet project to the figure's bottom: y + h / 2 screens (in px of the frame)
	const ndc = p.pos.clone().project(cam);
	near(((1 - ndc.y) / 2) * f.H, 0.6 * f.H + (0.3 * 844) / 2, 1e-6);
});

test('placing and its inverse: a grounded figure comes back exactly', () => {
	const cam = highCamera();
	const fig = { x: 0.31, y: 0.55, h: 0.22 };
	const p = placeFigure(cam, f, fig, 1.8, false);
	const back = screenPlace(cam, f, p.pos, p, yawToward(cam, p.pos, 25));
	near(back.x, fig.x);
	near(back.y, fig.y);
	near(back.h, fig.h);
	near(back.turn, 25, 1e-9);
	assert.equal(back.float, false);
});

test('a floating figure (eye level: its feet ray looks up) is placed along its ray and comes back', () => {
	const cam = eyeCamera();
	const fig = { x: 0.7, y: 0.2, h: 0.15 };
	const p = placeFigure(cam, f, fig, 1.8, false);
	assert.equal(p.grounded, false);
	const back = screenPlace(cam, f, p.pos, p, yawToward(cam, p.pos, 0));
	near(back.x, fig.x);
	near(back.y, fig.y);
	near(back.h, fig.h);
});

test('ground: false keeps a figure in the air; a close-up / airborne one is free', () => {
	const cam = highCamera();
	assert.equal(placeFigure(cam, f, { x: 0.5, y: 0.6, h: 0.3, ground: false }, 1.8, false).grounded, false);
	const free = placeFigure(cam, f, { x: 0.5, y: 0.6, h: 0.3 }, 0.3, true);
	assert.equal(free.grounded, false);
	assert.equal(screenPlace(cam, f, free.pos, free, 0).float, false); // free figures never ask for ground: false
});

test('a floating figure that another camera would ground says float (→ ground: false)', () => {
	const eye = eyeCamera();
	const p = placeFigure(eye, f, { x: 0.5, y: 0.2, h: 0.15 }, 1.8, false);
	assert.equal(p.grounded, false);
	// the same world point seen from high up: its feet ray goes down
	const high = highCamera();
	assert.equal(screenPlace(high, f, p.pos, p, 0).float, true);
});

// core.js casts an orthographic camera's rays from its position too (not parallel) – kept as it is: only the height
// round-trips exactly there (the isometric projection's x / y move a little on a rebuild, as in the studio)
test('an orthographic camera: the height from the scale and the view height', () => {
	const cam = new OrthographicCamera(-3, 3, 4, -4, 0.1, 400);
	cam.position.set(6, 6, 6);
	cam.lookAt(0, 0.8, 0);
	cam.updateMatrixWorld();
	const p = placeFigure(cam, f, { x: 0.4, y: 0.5, h: 0.25 }, 1.8, false);
	assert.equal(p.grounded, false);
	near(screenPlace(cam, f, p.pos, p, 0).h, 0.25);
});

test('applyPlace: rounded fields, ground only when floating, the turn only when the facing does not give it', () => {
	const fig: FigurePlace = { x: 0.5, y: 0.5, h: 0.3, facing: 'left', ground: false };
	applyPlace(fig, { x: 0.123456, y: 0.654321, h: 0.2, turn: -90.02, float: false });
	assert.deepEqual({ ...fig }, { x: 0.1235, y: 0.6543, h: 0.2, facing: 'left' });
	applyPlace(fig, { x: 0.1, y: 0.2, h: 0.3, turn: 30, float: true });
	assert.equal(fig.ground, false);
	assert.equal(fig.turn, 30);
	assert.equal(turnOf(fig), 30);
	assert.equal(turnOf({ facing: 'away' }), 180);
});

test('angles wrap into −180 … 180; the yaw of an orientation', () => {
	assert.equal(wrapDeg(190), -170);
	assert.equal(wrapDeg(-190), 170);
	assert.equal(wrapDeg(720 + 45), 45);
	near(yawOf(new Quaternion().setFromAxisAngle(new Vector3(0, 1, 0), 2.5)), 2.5);
});

test('the hand camera: rounded to mm and 0.1°; the key only changes with a real move', () => {
	assert.deepEqual(handCamera([1.23456, 2, 3.0004], [0, 0.99999, -1], 49.96), { pos: [1.235, 2, 3], target: [0, 1, -1], fov: 50 });
	assert.equal(cameraKey([1, 2, 3], [0, 0, 0]), cameraKey([1.0001, 2, 3], [0, 0, 0]));
	assert.notEqual(cameraKey([1, 2, 3], [0, 0, 0]), cameraKey([1.01, 2, 3], [0, 0, 0]));
});

test('the orbit centre: the stored target, else ahead at the subject depth (at least 1 m)', () => {
	const cam = eyeCamera();
	assert.deepEqual(orbitCentre(cam.position, cam.quaternion, null, [1, 2, 3]).toArray(), [1, 2, 3]);
	const c = orbitCentre(cam.position, cam.quaternion, new Vector3(0.4, 0, -2.5));
	near(c.z, -2.5);
	near(c.x, 0);
	near(c.y, 1.55);
	const close = orbitCentre(cam.position, cam.quaternion, new Vector3(0, 0, 3.4));
	near(close.z, 2.5); // 1 m ahead at least
});

test('the frame fitted into the stage, sizes in rem', () => {
	assert.deepEqual(fitFrame(398, 2000, 390, 694), { w: 390, h: 694, scale: 1 });
	const tall = fitFrame(1000, 700, 390, 1248);
	assert.equal(tall.h, 692);
	assert.deepEqual(fitFrame(5000, 5000, 390, 624).scale, 2.5);
	assert.equal(fitFrame(10, 10, 390, 624).w, 32);
	assert.equal(rem(390, 16), '24.375rem');
	assert.equal(rem(20, 0), '1.25rem');
});

test('the figure gestures: wheel size about the feet, Alt + wheel 15°, the Turn tool, a move on the ground', () => {
	assert.equal(wheelFactor(-100), 1.05);
	near(wheelFactor(100) * 1.05, 1);
	near(wheelTurn(100, 0), 15 * D);
	near(wheelTurn(0, -3), -15 * D);
	near(dragTurn(1, 50), 1.5);
	const anchor = new Vector3(1, 0, 1);
	assert.deepEqual(scaleAbout(new Vector3(1, 0.5, 1), anchor, 2).toArray(), [1, 1, 1]);
	assert.deepEqual(moveOnGround(new Vector3(0, 0.3, 0), new Vector3(1, 0, 1), new Vector3(2, 0, 4)).toArray(), [1, 0.3, 3]);
	// a turn about the feet: the anchor's world point stays put
	const pivot = new Group();
	pivot.position.set(2, 1, -1);
	const feet = new Vector3(0.3, -1, 0.2);
	pivot.updateMatrixWorld();
	const before = pivot.localToWorld(feet.clone());
	turnAbout(pivot, feet, 1.2);
	const after = pivot.localToWorld(feet.clone());
	near(after.distanceTo(before), 0, 1e-9);
	near(yawOf(pivot.quaternion), 1.2);
});
