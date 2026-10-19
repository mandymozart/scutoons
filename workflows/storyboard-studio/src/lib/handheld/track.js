// The handheld camera's maths (research/HANDHELD-CAMERA.md): the phone's pose → the shot camera (relative to an
// origin), the one smoothing filter (live and offline), a take's track sampled at any time. Plain three.js maths –
// shared by the PC page, the phone page, the renderer and the exports.
//
// A sample: { t (s), position [x, y, z] (m, the scene's world: Y up, as glTF), quaternion [x, y, z, w], fov (°) }.
import * as THREE from 'three';

const Y = new THREE.Vector3(0, 1, 0);

/** the heading of a camera orientation about the vertical (rad) – from its forward axis, its up axis when it looks straight down / up */
export function yawOf(q) {
	const f = new THREE.Vector3(0, 0, -1).applyQuaternion(q);
	if (Math.hypot(f.x, f.z) < 0.2) {
		const u = new THREE.Vector3(0, 1, 0).applyQuaternion(q).multiplyScalar(f.y < 0 ? 1 : -1);
		return Math.atan2(-u.x, -u.z);
	}
	return Math.atan2(-f.x, -f.z);
}

/**
 * "Set origin": the phone's pose now = the shot camera's pose now. Only the heading is matched – up stays up (both
 * worlds are gravity-aligned), so walking forward on the floor moves the camera forward on the scene's ground.
 * → plain arrays (it travels over the socket)
 */
export function makeOrigin(phonePos, phoneQuat, camPos, camQuat) {
	const q0 = new THREE.Quaternion().fromArray(phoneQuat);
	const qc = new THREE.Quaternion().fromArray(camQuat);
	const turn = new THREE.Quaternion().setFromAxisAngle(Y, yawOf(qc) - yawOf(q0));
	return { phonePos: [...phonePos], phoneQuat: q0.toArray(), camPos: [...camPos], camQuat: qc.toArray(), turn: turn.toArray() };
}

/** the phone's pose → the camera's pose (scale: phone metres → scene metres) */
export function mapPose(origin, phonePos, phoneQuat, scale = 1) {
	const turn = new THREE.Quaternion().fromArray(origin.turn);
	const p = new THREE.Vector3().fromArray(phonePos).sub(new THREE.Vector3().fromArray(origin.phonePos)).applyQuaternion(turn).multiplyScalar(scale);
	const position = new THREE.Vector3().fromArray(origin.camPos).add(p);
	// the phone's rotation since the origin, in the world, turned into the scene's heading
	const delta = new THREE.Quaternion().fromArray(phoneQuat).multiply(new THREE.Quaternion().fromArray(origin.phoneQuat).invert());
	const q = turn.clone().multiply(delta).multiply(turn.clone().invert()).multiply(new THREE.Quaternion().fromArray(origin.camQuat));
	return { position: position.toArray(), quaternion: q.normalize().toArray() };
}

/** DeviceOrientationEvent angles (°) + the screen's angle → the phone's orientation (as three's DeviceOrientationControls) */
export function orientationQuat(alpha, beta, gamma, screen = 0) {
	const D = Math.PI / 180;
	const q = new THREE.Quaternion().setFromEuler(new THREE.Euler(beta * D, alpha * D, -gamma * D, 'YXZ'));
	q.multiply(new THREE.Quaternion(-Math.SQRT1_2, 0, 0, Math.SQRT1_2)); // the camera looks out of the back, not down
	q.multiply(new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 0, 1), -screen * D));
	return q.toArray();
}

/**
 * The one smoothing filter (0 = raw handheld … 1 = a slow steadicam): exponential, a = 1 − exp(−dt / τ), τ = amount × 0.5 s;
 * position lerp, quaternion slerp. Live: one step per pose (smoother().step); offline: smooth(samples, amount).
 */
export function smoother(amount) {
	const tau = Math.max(0, amount) * 0.5;
	let p = null, q = null, t = null;
	return {
		step(s) {
			if (!p || tau === 0) {
				p = new THREE.Vector3().fromArray(s.position);
				q = new THREE.Quaternion().fromArray(s.quaternion);
			} else {
				const a = 1 - Math.exp(-Math.max(0, s.t - t) / tau);
				p.lerp(new THREE.Vector3().fromArray(s.position), a);
				q.slerp(new THREE.Quaternion().fromArray(s.quaternion), a);
			}
			t = s.t;
			return { ...s, position: p.toArray(), quaternion: q.toArray() };
		}
	};
}
export function smooth(samples, amount) {
	const f = smoother(amount);
	return samples.map((s) => f.step(s));
}

/** the track at time t (s): linear between samples, slerp for the rotation; clamped to its ends */
export function sampleAt(samples, t) {
	if (!samples.length) return null;
	if (t <= samples[0].t) return samples[0];
	const last = samples[samples.length - 1];
	if (t >= last.t) return last;
	let lo = 0, hi = samples.length - 1;
	while (hi - lo > 1) { const m = (lo + hi) >> 1; if (samples[m].t <= t) lo = m; else hi = m; }
	const a = samples[lo], b = samples[hi];
	const k = (t - a.t) / Math.max(1e-6, b.t - a.t);
	const p = new THREE.Vector3().fromArray(a.position).lerp(new THREE.Vector3().fromArray(b.position), k);
	const q = new THREE.Quaternion().fromArray(a.quaternion).slerp(new THREE.Quaternion().fromArray(b.quaternion), k);
	return { t, position: p.toArray(), quaternion: q.toArray(), fov: a.fov + ((b.fov ?? a.fov) - a.fov) * k };
}

/** a take as it is rendered / exported: smoothed, trimmed to in…out, resampled at fps, t from 0 */
export function bake(take, fps = 24) {
	const all = smooth(take.samples, take.smooth ?? 0);
	const end = all.length ? all[all.length - 1].t : 0;
	const tin = Math.max(0, take.in ?? 0), tout = Math.min(end, take.out ?? end);
	const n = Math.max(1, Math.floor((tout - tin) * fps + 1e-6) + 1);
	return Array.from({ length: n }, (_, i) => ({ ...sampleAt(all, tin + i / fps), t: +(i / fps).toFixed(4) }));
}

const r = (n, d) => +n.toFixed(d);
/** a sample as it is stored (rounded: 0.1 mm, 1e-5) */
export const packSample = (s) => ({ t: r(s.t, 4), position: s.position.map((n) => r(n, 4)), quaternion: s.quaternion.map((n) => r(n, 5)), fov: r(s.fov ?? 50, 2) });
