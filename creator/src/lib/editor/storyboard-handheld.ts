// The handheld camera's maths (#126; research/HANDHELD-CAMERA.md, the studio's lib/handheld/track.js): the phone's pose
// → the shot camera (relative to an origin, "Recenter"), the one smoothing filter (live and offline), a take's track at
// any time, the take as it is rendered (smoothed, trimmed, at fps). Plain quaternion maths without three.js; the tests
// compare it with three's. The phone's side (its orientation, the drag) is the one handheld's (@scutoons/content/storyboard).
//
// A sample: { t (s), position [x, y, z] (m, the scene's world: Y up, as glTF), quaternion [x, y, z, w], fov (°) }.
import { qAxisAngle, qMul, qRotate, yawOf, type Quat, type TakeSample, type Vec3 } from '@scutoons/content/storyboard';

export { DRAG_M_PER_PX, dragStep, orientationQuat, qAxisAngle, qEulerYXZ, qMul, qRotate, yawOf, type Quat, type Vec3 } from '@scutoons/content/storyboard';

export interface Pose {
	position: Vec3;
	quaternion: Quat;
}

/** the inverse of a unit quaternion */
export const qInv = (q: Quat): Quat => [-q[0], -q[1], -q[2], q[3]];
export function qNorm(q: Quat): Quat {
	const l = Math.hypot(...q) || 1;
	return [q[0] / l, q[1] / l, q[2] / l, q[3] / l];
}
/** the shortest way from a to b (three's slerp) */
export function qSlerp(a: Quat, b: Quat, t: number): Quat {
	if (t <= 0) return [...a];
	if (t >= 1) return [...b];
	let cos = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
	let c: Quat = [...b];
	if (cos < 0) {
		cos = -cos;
		c = [-b[0], -b[1], -b[2], -b[3]];
	}
	if (cos >= 1) return [...a];
	const sqrSin = 1 - cos * cos;
	if (sqrSin <= Number.EPSILON) {
		const s = 1 - t;
		return qNorm([s * a[0] + t * c[0], s * a[1] + t * c[1], s * a[2] + t * c[2], s * a[3] + t * c[3]]);
	}
	const sin = Math.sqrt(sqrSin);
	const half = Math.atan2(sin, cos);
	const ra = Math.sin((1 - t) * half) / sin,
		rb = Math.sin(t * half) / sin;
	return [a[0] * ra + c[0] * rb, a[1] * ra + c[1] * rb, a[2] * ra + c[2] * rb, a[3] * ra + c[3] * rb];
}
const lerp3 = (a: Vec3, b: Vec3, k: number): Vec3 => [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k];

/** "Recenter": the phone's pose now = the shot camera's pose now – only the heading is matched, up stays up (both worlds
 *  are gravity-aligned), so walking forward on the floor moves the camera forward on the scene's ground */
export interface Origin {
	phonePos: Vec3;
	phoneQuat: Quat;
	camPos: Vec3;
	camQuat: Quat;
	turn: Quat;
}
export function makeOrigin(phonePos: Vec3, phoneQuat: Quat, camPos: Vec3, camQuat: Quat): Origin {
	const turn = qAxisAngle([0, 1, 0], yawOf(camQuat) - yawOf(phoneQuat));
	return { phonePos: [...phonePos], phoneQuat: [...phoneQuat], camPos: [...camPos], camQuat: [...camQuat], turn };
}

/** the phone's pose → the camera's pose (scale: scene metres per phone metre) */
export function mapPose(origin: Origin, phonePos: Vec3, phoneQuat: Quat, scale = 1): Pose {
	const d = qRotate(origin.turn, [phonePos[0] - origin.phonePos[0], phonePos[1] - origin.phonePos[1], phonePos[2] - origin.phonePos[2]]);
	const position: Vec3 = [origin.camPos[0] + d[0] * scale, origin.camPos[1] + d[1] * scale, origin.camPos[2] + d[2] * scale];
	// the phone's rotation since the origin, in the world, turned into the scene's heading
	const delta = qMul(phoneQuat, qInv(origin.phoneQuat));
	const q = qMul(qMul(qMul(origin.turn, delta), qInv(origin.turn)), origin.camQuat);
	return { position, quaternion: qNorm(q) };
}

/**
 * The one smoothing filter (0 = raw handheld … 1 = a slow steadicam): exponential, a = 1 − exp(−dt / τ), τ = amount × 0.5 s;
 * position lerp, quaternion slerp. Live: one step per pose (smoother().step); offline: smooth(samples, amount).
 */
export function smoother(amount: number) {
	const tau = Math.max(0, amount) * 0.5;
	let p: Vec3 | null = null,
		q: Quat | null = null,
		t = 0;
	return {
		step(s: TakeSample): TakeSample {
			if (!p || !q || tau === 0) {
				p = [...s.position];
				q = [...s.quaternion];
			} else {
				const a = 1 - Math.exp(-Math.max(0, s.t - t) / tau);
				p = lerp3(p, s.position, a);
				q = qSlerp(q, s.quaternion, a);
			}
			t = s.t;
			return { ...s, position: [...p], quaternion: [...q] };
		}
	};
}
export function smooth(samples: readonly TakeSample[], amount: number): TakeSample[] {
	const f = smoother(amount);
	return samples.map((s) => f.step(s));
}

/** the track at time t (s): linear between samples, slerp for the rotation; clamped to its ends */
export function sampleAt(samples: readonly TakeSample[], t: number): TakeSample | null {
	if (!samples.length) return null;
	if (t <= samples[0].t) return samples[0];
	const last = samples[samples.length - 1];
	if (t >= last.t) return last;
	let lo = 0,
		hi = samples.length - 1;
	while (hi - lo > 1) {
		const m = (lo + hi) >> 1;
		if (samples[m].t <= t) lo = m;
		else hi = m;
	}
	const a = samples[lo],
		b = samples[hi];
	const k = (t - a.t) / Math.max(1e-6, b.t - a.t);
	const fa = a.fov ?? 50;
	return { t, position: lerp3(a.position, b.position, k), quaternion: qSlerp(a.quaternion, b.quaternion, k), fov: fa + ((b.fov ?? fa) - fa) * k };
}

/** what bake needs of a take */
export interface TakeTrack {
	samples: TakeSample[];
	smooth?: number;
	in?: number;
	out?: number;
}

/** a take as it is rendered / exported: smoothed, trimmed to in…out, resampled at fps, t from 0 */
export function bake(take: TakeTrack, fps = 24): TakeSample[] {
	const all = smooth(take.samples, take.smooth ?? 0);
	if (!all.length) return [];
	const end = all[all.length - 1].t;
	const tin = Math.max(0, take.in ?? 0),
		tout = Math.min(end, take.out ?? end);
	const n = Math.max(1, Math.floor((tout - tin) * fps + 1e-6) + 1);
	return Array.from({ length: n }, (_, i) => ({ ...sampleAt(all, tin + i / fps)!, t: +(i / fps).toFixed(4) }));
}

const r = (n: number, d: number) => +n.toFixed(d);
/** a sample as it is stored (rounded: 0.1 mm, 1e-5) – a take of a minute at 60 Hz stays a few hundred KB */
export const packSample = (s: TakeSample): TakeSample => ({
	t: r(s.t, 4),
	position: s.position.map((n) => r(n, 4)) as Vec3,
	quaternion: s.quaternion.map((n) => r(n, 5)) as Quat,
	fov: r(s.fov ?? 50, 2)
});
