// The phone's side of the handheld camera (#126; the studio's Phone.svelte – research/HANDHELD-CAMERA.md §2, STUDIO-IN-
// CREATOR §2.6): the one-time code from the QR → the PC's offer → our answer → the data channels; then the phone's pose
// at the sensor's rate to the PC, peer to peer – the PC maps it onto the shot camera, smooths, shows and records (the
// phone renders nothing: "only the camera, the pose sensor and the channel"). WebXR `immersive-ar` (ARCore: position +
// rotation) where the phone has it; else DeviceOrientation (rotation; iOS asks on a tap) + a drag for the position.
// Record / Stop and Recenter go to the PC as messages. No session, no paraglide, no three: the barebone page's logic.
import { API_URL } from '$lib/config.ts';
import { dragStep, orientationQuat, type Quat, type Vec3 } from '$lib/editor/storyboard-handheld.ts';
import { decodePc, ICE_SERVERS, makeAnswer, PairingError, publicCalls, send, CODE, type Channels, type PairingCalls, type PhoneMode } from '$lib/editor/storyboard-pairing.ts';

export type PhoneStatus = 'nocode' | 'connecting' | 'connected' | 'expired' | 'failed' | 'closed';

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

export class PhoneSession {
	status = $state<PhoneStatus>('connecting');
	shot = $state<string | null>(null);
	/** the PC says the handheld camera is on for its shot */
	on = $state(true);
	recording = $state(false);
	seconds = $state(0);
	mode = $state<PhoneMode>('');
	tracking = $state(true);
	/** WebXR immersive-ar is there */
	xr = $state(false);
	error = $state('');

	private pc: RTCPeerConnection | null = null;
	private ch: Channels | null = null;
	private drag: Vec3 = [0, 0, 0];
	private orient: Quat | null = null;
	private session: XrSession | null = null;
	private wake: { release(): Promise<void> } | null = null;
	private readonly pointers = new Map<number, { x: number; y: number }>();

	constructor(
		readonly code: string,
		private readonly calls: PairingCalls = publicCalls(API_URL)
	) {
		if (!CODE.test(code)) this.status = 'nocode';
	}

	async connect(): Promise<void> {
		if (this.status === 'nocode') return;
		const xr = (navigator as unknown as { xr?: XrSystem }).xr;
		void xr?.isSessionSupported('immersive-ar').then((ok) => (this.xr = ok), () => undefined);
		try {
			const pc = (this.pc = new RTCPeerConnection({ iceServers: ICE_SERVERS }));
			pc.addEventListener('connectionstatechange', () => {
				if (pc.connectionState === 'failed') this.status = 'failed';
				if (pc.connectionState === 'closed' || pc.connectionState === 'disconnected') this.status = this.status === 'connected' ? 'closed' : this.status;
			});
			const { view, channels } = await makeAnswer(pc, this.calls, this.code);
			this.shot = view.shot;
			this.ch = await channels;
			this.ch.ctl.addEventListener('message', (e) => this.onMessage(e.data));
			const ready = () => {
				this.status = 'connected';
				send(this.ch?.ctl, { type: 'hello', ua: navigator.userAgent });
				this.sendState();
			};
			if (this.ch.ctl.readyState === 'open') ready();
			else this.ch.ctl.addEventListener('open', ready);
			this.ch.ctl.addEventListener('close', () => (this.status = 'closed'));
		} catch (e) {
			this.status = e instanceof PairingError && (e.status === 404 || e.status === 409) ? 'expired' : 'failed';
			this.error = e instanceof Error ? e.message : String(e);
		}
	}

	private onMessage(data: unknown): void {
		const m = decodePc(data);
		if (!m) return;
		this.shot = m.shot;
		this.on = m.on;
		this.recording = m.recording;
		this.seconds = m.seconds;
	}
	private sendState(): void {
		send(this.ch?.ctl, { type: 'state', mode: this.mode, tracking: this.tracking });
	}
	/** a pose of the phone (its own world) at the sensor's rate */
	private pose(p: Vec3, q: Quat): void {
		send(this.ch?.pose, { type: 'pose', t: performance.now(), p, q });
	}

	record = () => send(this.ch?.ctl, { type: 'rec', on: !this.recording });
	recenter = () => send(this.ch?.ctl, { type: 'recenter' });

	// ── Rotation only: DeviceOrientation + drag ─────────────────────────────────────────────────────────────

	private readonly onOrient = (e: DeviceOrientationEvent) => {
		if (e.alpha == null) return;
		this.orient = orientationQuat(e.alpha, e.beta ?? 0, e.gamma ?? 0, screen.orientation?.angle ?? 0);
		this.pose([...this.drag], this.orient);
	};
	/** a tap: iOS asks for the motion sensor here (it must be a tap) */
	async startOrientation(): Promise<void> {
		const ask = (globalThis.DeviceOrientationEvent as unknown as { requestPermission?: () => Promise<PermissionState> } | undefined)?.requestPermission;
		try {
			if (typeof ask === 'function' && (await ask()) !== 'granted') {
				this.error = 'denied';
				return;
			}
		} catch {
			// an older engine without the question
		}
		this.error = '';
		addEventListener('deviceorientation', this.onOrient);
		this.mode = 'orientation';
		this.sendState();
		void this.keepAwake();
	}
	down = (e: PointerEvent) => {
		if (this.mode !== 'orientation') return;
		this.pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
		(e.currentTarget as Element | null)?.setPointerCapture?.(e.pointerId);
	};
	move = (e: PointerEvent) => {
		const last = this.pointers.get(e.pointerId);
		if (!last || !this.orient) return;
		this.pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
		this.drag = dragStep(this.drag, this.orient, e.clientX - last.x, e.clientY - last.y, this.pointers.size);
		this.pose([...this.drag], this.orient);
	};
	up = (e: PointerEvent) => void this.pointers.delete(e.pointerId);

	// ── WebXR: ARCore's 6DoF pose; the camera image through our page (dom-overlay) ─────────────────────────────

	async startXR(root: HTMLElement): Promise<void> {
		const xr = (navigator as unknown as { xr?: XrSystem }).xr;
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
			this.mode = '';
			this.sendState();
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
			}
			if (!pose) return;
			const { position: p, orientation: o } = pose.transform;
			this.pose([p.x, p.y, p.z], [o.x, o.y, o.z, o.w]);
		};
		session.requestAnimationFrame(frame);
		this.sendState();
		void this.keepAwake();
	}

	private async keepAwake(): Promise<void> {
		try {
			this.wake = (await (navigator as unknown as { wakeLock?: { request(t: 'screen'): Promise<{ release(): Promise<void> }> } }).wakeLock?.request('screen')) ?? null;
		} catch {
			// the screen may dim
		}
	}

	close(): void {
		removeEventListener('deviceorientation', this.onOrient);
		void this.session?.end().catch(() => undefined);
		void this.wake?.release().catch(() => undefined);
		this.pc?.close();
		this.pc = null;
	}
}
