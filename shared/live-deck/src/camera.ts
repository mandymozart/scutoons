// camera.ts (research/LIVE-CAMERAS.md): a reader's phone holding a live camera, on the handheld's link (hand.ts) as role
// `reader`. The core decides the hold (asked, held, taken over, who has it), takes the camera again by itself after a drop
// or a reload while nobody else claimed it, and words the relay's refusal; this hands the person's yes, Take it back and
// Leave in and the core's words out. The phone's motion is the one handheld (@scutoons/content/storyboard `Handheld`).
import type { LiveCamera } from '@scutoons/live';
import { HandLink, type HandCore, type HandLinkOptions } from './hand.ts';

/** a phone's hold on a live camera, the core's word (deck-space live_cameras `Holding`) */
export interface Holding {
	hold: 'none' | 'asked' | 'held' | 'taken_over';
	/** the camera it holds, asked for or lost (a forced switch names the new one) */
	camera?: string;
	/** another phone holds that camera now: its name, else its client id */
	other?: string;
	/** why the relay refused the last claim */
	refused?: string;
}

interface CameraCore extends HandCore {
	camera_claim(camera: string): boolean;
	camera_resume(camera: string, was: string): boolean;
	camera_leave(): boolean;
}

export interface CameraLinkOptions extends Omit<HandLinkOptions, 'role' | 'token' | 'onOther'> {
	/** the phone's own secret, kept in its browser */
	key: string;
	/** its hold changed; the cameras it may take, with who holds each; the conductor's flash for its camera */
	onHolding?: (h: Holding) => void;
	onCameras?: (cameras: LiveCamera[]) => void;
	onFlash?: (camera: string, settings: Record<string, unknown>) => void;
}

type Move = (core: CameraCore) => void;

export class CameraLink {
	/** the handheld's link under it: the one handheld's sink (pose, state), Recenter, close */
	readonly link: HandLink;
	/** what the person did before the core was there: handed in at its first status */
	private early: Move[] = [];

	constructor(o: CameraLinkOptions) {
		this.link = new HandLink({
			...o,
			role: 'reader',
			onStatus: (s, rung, line) => {
				this.flush();
				o.onStatus?.(s, rung, line);
			},
			onOther: (w) => {
				if ('holding' in w) o.onHolding?.(w.holding);
				else if ('cameras' in w) o.onCameras?.(w.cameras);
				else if ('camera_flash' in w) o.onFlash?.(w.camera_flash['camera'], w.camera_flash['settings']);
			}
		});
	}

	private move(f: Move): void {
		this.early.push(f);
		this.flush();
	}

	private flush(): void {
		const s = this.link.s;
		if (!s || !this.early.length) return;
		for (const f of this.early.splice(0)) f(s.core as CameraCore);
		s.run();
	}

	/** the person said yes to `camera` (or Take it back) */
	claim(camera: string): void {
		this.move((c) => c.camera_claim(camera));
	}

	/** after a reload: the page held `camera` as the client `was` (its browser kept both) */
	resume(camera: string, was: string): void {
		this.move((c) => c.camera_resume(camera, was));
	}

	/** Leave */
	leave(): void {
		this.move((c) => c.camera_leave());
	}
}
