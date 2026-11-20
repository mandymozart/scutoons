// The core's follow of a phone (relay-web's TakeFollow, deck-space dop/take.rs) as plain values for the storyboard: the poses go in,
// the camera now and the take's keys come out. Only the wasm's arrays and JSON text become objects here; the core decides the rest.
import type { FollowCore } from './hand-port.ts';

type Vec3 = [number, number, number];
type Quat = [number, number, number, number];
export interface Key {
	t: number;
	position: Vec3;
	quaternion: Quat;
}

export interface FollowPort {
	/** false: late or malformed – dropped */
	pose(t: number, p: readonly number[], q: readonly number[]): boolean;
	/** the phone's next pose = the camera now */
	recenter(): void;
	/** Record (a new take from the next pose) and Stop */
	keep(on: boolean): void;
	/** how long the take runs so far (s) */
	secs(): number;
	/** the camera now (null before the first pose) */
	held(): { position: Vec3; quaternion: Quat } | null;
	/** the take's keys, s from its first pose, thinned by the core; null: too short to be a move */
	take(): Key[] | null;
	free(): void;
}

export function followPort(core: FollowCore): FollowPort {
	return {
		pose: (t, p, q) => core.pose(t, Float64Array.from(p), Float64Array.from(q)),
		recenter: () => core.recenter(),
		keep: (on) => core.keep(on),
		secs: () => core.secs(),
		held() {
			const h = core.held();
			return h.length === 7 ? { position: [h[0], h[1], h[2]], quaternion: [h[3], h[4], h[5], h[6]] } : null;
		},
		take() {
			const json = core.take();
			return json ? (JSON.parse(json) as Key[]) : null;
		},
		free: () => core.free()
	};
}
