// handheld.ts (research/LIVE-CAMERAS.md §2; HANDHELD-CAMERA.md): the one handheld – a phone's motion as a
// camera's pose stream. The creator's storyboard phone page (/h/<code>) and the reader's camera page (/live/cameras/<id>)
// both use it; what the pose steers is the other end's (the storyboard's PC, deck-space's camera). WebXR `immersive-ar`
// (ARCore: position + rotation) where the phone has it, else DeviceOrientation (rotation; iOS asks on a tap) and a drag for
// the position; the screen kept awake while it streams. It renders nothing ("only the camera, the pose sensor and the
// channel") and holds no link: the page hands in where the poses go (`HandheldSink`: a HandLink).

export type Vec3 = [number, number, number];
export type Quat = [number, number, number, number];
/** what the phone tracks: WebXR (position + rotation), the orientation sensor (rotation only), nothing yet */
export type HandMode = 'xr' | 'orientation' | 'none';

const D = Math.PI / 180;

/** a · b (three's multiplyQuaternions) */
export function qMul(a: Quat, b: Quat): Quat {
	const [ax, ay, az, aw] = a,
		[bx, by, bz, bw] = b;
	return [ax * bw + aw * bx + ay * bz - az * by, ay * bw + aw * by + az * bx - ax * bz, az * bw + aw * bz + ax * by - ay * bx, aw * bw - ax * bx - ay * by - az * bz];
}
export function qAxisAngle(axis: Vec3, angle: number): Quat {
	const s = Math.sin(angle / 2);
	return [axis[0] * s, axis[1] * s, axis[2] * s, Math.cos(angle / 2)];
}
/** Euler angles (rad) in three's 'YXZ' order */
export function qEulerYXZ(x: number, y: number, z: number): Quat {
	const [c1, c2, c3] = [Math.cos(x / 2), Math.cos(y / 2), Math.cos(z / 2)];
	const [s1, s2, s3] = [Math.sin(x / 2), Math.sin(y / 2), Math.sin(z / 2)];
	return [s1 * c2 * c3 + c1 * s2 * s3, c1 * s2 * c3 - s1 * c2 * s3, c1 * c2 * s3 - s1 * s2 * c3, c1 * c2 * c3 + s1 * s2 * s3];
}
/** v turned by q */
export function qRotate(q: Quat, v: Vec3): Vec3 {
	const [x, y, z, w] = q;
	const tx = 2 * (y * v[2] - z * v[1]),
		ty = 2 * (z * v[0] - x * v[2]),
		tz = 2 * (x * v[1] - y * v[0]);
	return [v[0] + w * tx + y * tz - z * ty, v[1] + w * ty + z * tx - x * tz, v[2] + w * tz + x * ty - y * tx];
}

/** the heading of a camera orientation about the vertical (rad) – from its forward axis, its up axis when it looks straight down / up */
export function yawOf(q: Quat): number {
	const f = qRotate(q, [0, 0, -1]);
	if (Math.hypot(f[0], f[2]) < 0.2) {
		const k = f[1] < 0 ? 1 : -1;
		const u = qRotate(q, [0, 1, 0]).map((n) => n * k);
		return Math.atan2(-u[0], -u[2]);
	}
	return Math.atan2(-f[0], -f[2]);
}

/** DeviceOrientationEvent angles (°) + the screen's angle → the phone's orientation (as three's DeviceOrientationControls) */
export function orientationQuat(alpha: number, beta: number, gamma: number, screen = 0): Quat {
	let q = qEulerYXZ(beta * D, alpha * D, -gamma * D);
	q = qMul(q, [-Math.SQRT1_2, 0, 0, Math.SQRT1_2]); // the camera looks out of the back, not down
	return qMul(q, qAxisAngle([0, 0, 1], -screen * D));
}

/** metres per CSS px of a drag on the phone (rotation-only mode) */
export const DRAG_M_PER_PX = 0.01;
/**
 * Rotation only (no WebXR): the position by dragging – one finger moves on the ground along the phone's heading (up =
 * forward), two fingers move up / down (each finger half). → the new position
 */
export function dragStep(at: Vec3, phoneQuat: Quat, dx: number, dy: number, fingers: number): Vec3 {
	const k = DRAG_M_PER_PX / Math.max(1, fingers);
	if (fingers >= 2) return [at[0], at[1] - dy * k, at[2]];
	const yaw = yawOf(phoneQuat);
	const fwd: Vec3 = [-Math.sin(yaw), 0, -Math.cos(yaw)],
		right: Vec3 = [Math.cos(yaw), 0, -Math.sin(yaw)];
	return [at[0] + right[0] * dx * k - fwd[0] * dy * k, at[1], at[2] + right[2] * dx * k - fwd[2] * dy * k];
}

/** where the handheld's poses go (@scutoons/live-deck's HandLink is one) */
export interface HandheldSink {
	/** the phone's pose at `t` (its clock, ms): position (3), rotation (quaternion, 4) */
	pose(t: number, p: readonly number[], q: readonly number[]): void;
	/** what it tracks (sent again whenever it changes) */
	state(s: { mode: HandMode; tracking: boolean; ua?: string }): void;
}

/** the WebXR parts used here (the DOM lib has no WebXR types) */
interface XrPose {
	emulatedPosition: boolean;
	transform: { position: { x: number; y: number; z: number }; orientation: { x: number; y: number; z: number; w: number } };
}
interface XrFrame {
	getViewerPose(space: unknown): XrPose | null;
}
interface XrSession {
	domOverlayState?: unknown;
	renderState: { baseLayer?: { framebuffer: WebGLFramebuffer | null } };
	updateRenderState(s: { baseLayer: unknown }): void;
	requestReferenceSpace(type: string): Promise<unknown>;
	requestAnimationFrame(cb: (time: number, frame: XrFrame) => void): number;
	addEventListener(type: 'end', fn: () => void): void;
	end(): Promise<void>;
}
interface XrSystem {
	isSessionSupported(mode: string): Promise<boolean>;
	requestSession(mode: string, init: object): Promise<XrSession>;
}

/** what the handheld takes from the browser (the page's globals; a test hands in its own) */
export interface HandheldEnv {
	addEventListener(type: 'deviceorientation', fn: (e: { alpha: number | null; beta: number | null; gamma: number | null }) => void): void;
	removeEventListener(type: 'deviceorientation', fn: (e: { alpha: number | null; beta: number | null; gamma: number | null }) => void): void;
	now(): number;
	/** the screen's angle (°) */
	screenAngle(): number;
	ua: string;
	xr?: XrSystem;
	/** iOS: the motion sensor's question (asked on a tap) */
	askMotion?: () => Promise<string>;
	wakeLock?: { request(t: 'screen'): Promise<{ release(): Promise<void> }> };
}

function browserEnv(): HandheldEnv {
	const g = globalThis as unknown as Window & { DeviceOrientationEvent?: { requestPermission?: () => Promise<string> } };
	const nav = (typeof navigator === 'undefined' ? {} : navigator) as Navigator & { xr?: XrSystem; wakeLock?: HandheldEnv['wakeLock'] };
	const ask = g.DeviceOrientationEvent?.requestPermission;
	return {
		addEventListener: (t, fn) => g.addEventListener(t, fn as unknown as EventListener),
		removeEventListener: (t, fn) => g.removeEventListener(t, fn as unknown as EventListener),
		now: () => performance.now(),
		screenAngle: () => g.screen?.orientation?.angle ?? 0,
		ua: nav.userAgent ?? '',
		xr: nav.xr,
		askMotion: typeof ask === 'function' ? () => ask.call(g.DeviceOrientationEvent) : undefined,
		wakeLock: nav.wakeLock
	};
}

/** The phone's motion as a pose stream into `sink`. `changed` runs whenever mode, tracking, xr or error change. */
export class Handheld {
	mode: HandMode = 'none';
	tracking = true;
	/** WebXR immersive-ar is there */
	xr = false;
	/** '' · 'denied' (the motion sensor's question was answered no) */
	error = '';

	private drag: Vec3 = [0, 0, 0];
	private orient: Quat | null = null;
	private session: XrSession | null = null;
	private wake: { release(): Promise<void> } | null = null;
	private readonly pointers = new Map<number, { x: number; y: number }>();
	private closed = false;

	constructor(
		private readonly sink: HandheldSink,
		private readonly changed: () => void = () => {},
		private readonly env: HandheldEnv = browserEnv()
	) {}

	/** asks whether WebXR immersive-ar is there (`xr`) */
	probe(): void {
		void this.env.xr?.isSessionSupported('immersive-ar').then(
			(ok) => {
				this.xr = ok;
				this.changed();
			},
			() => undefined
		);
	}

	/** what the phone tracks, to the sink (a page sends it again after its link joined) */
	sendState(): void {
		this.sink.state({ mode: this.mode, tracking: this.tracking, ua: this.env.ua });
	}

	private pose(p: Vec3, q: Quat): void {
		if (!this.closed) this.sink.pose(this.env.now(), p, q);
	}

	private readonly onOrient = (e: { alpha: number | null; beta: number | null; gamma: number | null }) => {
		if (e.alpha == null) return;
		this.orient = orientationQuat(e.alpha, e.beta ?? 0, e.gamma ?? 0, this.env.screenAngle());
		this.pose([...this.drag], this.orient);
	};

	/** a tap: rotation from the orientation sensor (iOS asks for it here – it must be a tap) */
	async startOrientation(): Promise<void> {
		try {
			if (this.env.askMotion && (await this.env.askMotion()) !== 'granted') {
				this.error = 'denied';
				this.changed();
				return;
			}
		} catch {
			// an older engine without the question
		}
		this.error = '';
		this.env.addEventListener('deviceorientation', this.onOrient);
		this.mode = 'orientation';
		this.sendState();
		this.changed();
		void this.keepAwake();
	}

	/** rotation only: a drag moves the position (one finger on the ground, two up and down) */
	down = (e: { pointerId: number; clientX: number; clientY: number; currentTarget?: EventTarget | null }) => {
		if (this.mode !== 'orientation') return;
		this.pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
		(e.currentTarget as Element | null)?.setPointerCapture?.(e.pointerId);
	};
	move = (e: { pointerId: number; clientX: number; clientY: number }) => {
		const last = this.pointers.get(e.pointerId);
		if (!last || !this.orient) return;
		this.pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
		this.drag = dragStep(this.drag, this.orient, e.clientX - last.x, e.clientY - last.y, this.pointers.size);
		this.pose([...this.drag], this.orient);
	};
	up = (e: { pointerId: number }) => void this.pointers.delete(e.pointerId);

	/** WebXR: ARCore's 6DoF pose, the camera image through the page (dom-overlay on `root`); else the orientation sensor */
	async startXR(root: HTMLElement): Promise<void> {
		const xr = this.env.xr;
		let session: XrSession;
		try {
			session = await xr!.requestSession('immersive-ar', { requiredFeatures: ['local'], optionalFeatures: ['dom-overlay'], domOverlay: { root } });
		} catch {
			return this.startOrientation();
		}
		if (!session.domOverlayState) {
			await session.end().catch(() => undefined);
			return this.startOrientation();
		}
		const gl = document.createElement('canvas').getContext('webgl', { xrCompatible: true } as WebGLContextAttributes)!;
		const Layer = (globalThis as unknown as { XRWebGLLayer: new (s: XrSession, gl: WebGLRenderingContext) => unknown }).XRWebGLLayer;
		session.updateRenderState({ baseLayer: new Layer(session, gl) });
		const space = await session.requestReferenceSpace('local');
		this.session = session;
		this.mode = 'xr';
		session.addEventListener('end', () => {
			this.session = null;
			this.mode = 'none';
			this.sendState();
			this.changed();
		});
		const frame = (_t: number, f: XrFrame) => {
			if (!this.session) return;
			session.requestAnimationFrame(frame);
			gl.bindFramebuffer(gl.FRAMEBUFFER, session.renderState.baseLayer?.framebuffer ?? null);
			gl.clearColor(0, 0, 0, 0);
			gl.clear(gl.COLOR_BUFFER_BIT);
			const pose = f.getViewerPose(space);
			const ok = !!pose && !pose.emulatedPosition;
			if (ok !== this.tracking) {
				this.tracking = ok;
				this.sendState();
				this.changed();
			}
			if (!pose) return;
			const { position: p, orientation: o } = pose.transform;
			this.pose([p.x, p.y, p.z], [o.x, o.y, o.z, o.w]);
		};
		session.requestAnimationFrame(frame);
		this.sendState();
		this.changed();
		void this.keepAwake();
	}

	private async keepAwake(): Promise<void> {
		try {
			this.wake = (await this.env.wakeLock?.request('screen')) ?? null;
		} catch {
			// the screen may dim
		}
	}

	close(): void {
		this.closed = true;
		this.env.removeEventListener('deviceorientation', this.onOrient);
		void this.session?.end().catch(() => undefined);
		void this.wake?.release().catch(() => undefined);
	}
}
