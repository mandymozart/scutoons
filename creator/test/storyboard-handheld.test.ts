// The handheld camera's maths (src/lib/editor/storyboard-handheld.ts, #126): the quaternion helpers against three.js (the
// studio's track.js did it with three; the phone's page must not load it), Recenter's origin, the pose mapping, the
// orientation sensor, the smoothing filter, sampling and baking a take.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as THREE from 'three';
import { bake, dragStep, makeOrigin, mapPose, orientationQuat, packSample, qEulerYXZ, qMul, qRotate, qSlerp, sampleAt, smooth, yawOf, type Quat, type Vec3 } from '../src/lib/editor/storyboard-handheld.ts';

const near = (a: readonly number[], b: readonly number[], eps = 1e-9, what = '') => a.forEach((x, i) => assert.ok(Math.abs(x - b[i]) < eps, `${what} [${i}] ${x} ≠ ${b[i]}`));
/** q and −q are the same rotation */
const sameRot = (a: Quat, b: readonly number[], eps = 1e-9) => (a[3] * b[3] + a[0] * b[0] + a[1] * b[1] + a[2] * b[2] < 0 ? near(a.map((n) => -n), b, eps) : near(a, b, eps));
const q3 = (q: Quat) => new THREE.Quaternion(...q);
const rnd = (seed: number) => () => ((seed = (seed * 16807) % 2147483647) / 2147483647) * 2 - 1;
const r = rnd(7);
const randQ = (): Quat => {
	const q = new THREE.Quaternion(r(), r(), r(), r()).normalize();
	return [q.x, q.y, q.z, q.w];
};

test('the quaternion helpers match three.js', () => {
	for (let i = 0; i < 20; i++) {
		const a = randQ(),
			b = randQ(),
			v: Vec3 = [r() * 5, r() * 5, r() * 5];
		near(qMul(a, b), q3(a).multiply(q3(b)).toArray(), 1e-12, 'mul');
		near(qRotate(a, v), new THREE.Vector3(...v).applyQuaternion(q3(a)).toArray(), 1e-9, 'rotate');
		for (const t of [0.1, 0.5, 0.93]) near(qSlerp(a, b, t), q3(a).slerp(q3(b), t).toArray(), 1e-9, 'slerp');
		const [x, y, z] = [r() * 3, r() * 3, r() * 3];
		near(qEulerYXZ(x, y, z), new THREE.Quaternion().setFromEuler(new THREE.Euler(x, y, z, 'YXZ')).toArray(), 1e-12, 'euler');
	}
});

test('yaw: the heading about the vertical – also looking straight down', () => {
	const turned = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), 0.7).toArray() as Quat;
	assert.ok(Math.abs(yawOf(turned) - 0.7) < 1e-9);
	const down = new THREE.Quaternion().setFromEuler(new THREE.Euler(-Math.PI / 2, 0.4, 0, 'YXZ')).toArray() as Quat;
	assert.ok(Math.abs(yawOf(down) - 0.4) < 1e-9, String(yawOf(down)));
});

test('Recenter: the phone\'s pose at the origin = the shot camera; walking forward moves the camera forward in the scene', () => {
	const cam = new THREE.Object3D();
	cam.position.set(2, 1.5, 6);
	cam.lookAt(0, 1, 0);
	const camQuat = cam.quaternion.toArray() as Quat;
	// the phone held upright, facing some other way in its own room
	const phoneQuat = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), 2.1).toArray() as Quat;
	const origin = makeOrigin([0.3, 1.2, -0.4], phoneQuat, cam.position.toArray() as Vec3, camQuat);
	const at = mapPose(origin, [0.3, 1.2, -0.4], phoneQuat);
	near(at.position, [2, 1.5, 6]);
	sameRot(at.quaternion, camQuat);
	// one metre along the phone's forward → one metre along the camera's heading on the ground (scale 2: two)
	const fwd = qRotate(phoneQuat, [0, 0, -1]);
	const moved = mapPose(origin, [0.3 + fwd[0], 1.2, -0.4 + fwd[2]], phoneQuat, 2);
	const yaw = yawOf(camQuat);
	near(moved.position, [2 - 2 * Math.sin(yaw), 1.5, 6 - 2 * Math.cos(yaw)], 1e-9);
	// lifting the phone lifts the camera (up stays up)
	near(mapPose(origin, [0.3, 1.7, -0.4], phoneQuat).position, [2, 2, 6]);
});

test('the orientation sensor: flat on its back = looking down; upright facing north = looking along −z', () => {
	const flat = orientationQuat(0, 0, 0);
	near(qRotate(flat, [0, 0, -1]), [0, -1, 0], 1e-12);
	const upright = orientationQuat(0, 90, 0);
	near(qRotate(upright, [0, 0, -1]), [0, 0, -1], 1e-12);
	// as three's DeviceOrientationControls (the studio's track.js), the screen's angle included
	const D = Math.PI / 180;
	for (const [a, b, g, s] of [[30, 70, -20, 90], [200, 10, 45, 0], [90, 90, 0, 270]]) {
		const q = new THREE.Quaternion().setFromEuler(new THREE.Euler(b * D, a * D, -g * D, 'YXZ'));
		q.multiply(new THREE.Quaternion(-Math.SQRT1_2, 0, 0, Math.SQRT1_2)).multiply(new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 0, 1), -s * D));
		near(orientationQuat(a, b, g, s), q.toArray(), 1e-12, `${a} ${b} ${g} ${s}`);
	}
});

test('a drag (rotation only): one finger up = forward along the heading, sideways = right; two fingers = up / down', () => {
	const north = orientationQuat(0, 90, 0);
	near(dragStep([0, 0, 0], north, 0, -100, 1), [0, 0, -1], 1e-9);
	near(dragStep([0, 0, 0], north, 100, 0, 1), [1, 0, 0], 1e-9);
	near(dragStep([0, 0, 0], north, 0, -100, 2), [0, 0.5, 0], 1e-9);
	// turned to the west (yaw +90°): forward is −x
	const west = orientationQuat(90, 90, 0);
	near(dragStep([0, 0, 0], west, 0, -100, 1), [-1, 0, 0], 1e-9);
});

const take = (n: number, dt = 0.1) => Array.from({ length: n }, (_, i) => ({ t: i * dt, position: [i, 0, 0] as Vec3, quaternion: [0, 0, 0, 1] as Quat, fov: 50 }));

test('smoothing: 0 = raw; more lags behind a step, the same live and offline', () => {
	const s = take(5);
	assert.deepEqual(smooth(s, 0).map((x) => x.position[0]), [0, 1, 2, 3, 4]);
	const soft = smooth(s, 1).map((x) => x.position[0]);
	assert.equal(soft[0], 0);
	assert.ok(soft[4] < 4 && soft[4] > soft[3]);
	// a = 1 − exp(−0.1 / 0.5)
	assert.ok(Math.abs(soft[1] - (1 - Math.exp(-0.2))) < 1e-12);
});

test('sampleAt: between samples, clamped at the ends', () => {
	const s = take(3);
	assert.equal(sampleAt(s, -1)!.position[0], 0);
	assert.equal(sampleAt(s, 9)!.position[0], 2);
	near(sampleAt(s, 0.15)!.position, [1.5, 0, 0], 1e-9);
	assert.equal(sampleAt([], 0), null);
});

test('bake: trimmed to in … out, at fps, t from 0', () => {
	const frames = bake({ samples: take(31), in: 1, out: 2 }, 10);
	assert.equal(frames.length, 11);
	assert.deepEqual([frames[0].t, frames[10].t], [0, 1]);
	near(frames[0].position, [10, 0, 0], 1e-9);
	near(frames[10].position, [20, 0, 0], 1e-9);
	assert.deepEqual(bake({ samples: [] }), []);
});

test('a stored sample is rounded', () => {
	assert.deepEqual(packSample({ t: 1.234567, position: [1.000049, 2, 3], quaternion: [0.1234567, 0, 0, 1], fov: 50.123 }), { t: 1.2346, position: [1, 2, 3], quaternion: [0.12346, 0, 0, 1], fov: 50.12 });
});
