// A reader's phone on a live session's cameras (research/LIVE-CAMERAS.md §3, §5, §5a): the core's link (@scutoons/live-deck
// CameraLink: the hold, the list, the flash, the claim taken again after a drop) and the one handheld (its motion as the
// pose stream). The page shows what they say; here only what the browser keeps (live-cameras.ts) and the page's state.
import type { LiveCamera } from '@scutoons/live';
import { CameraLink, loadRelay, type Holding } from '@scutoons/live-deck/hand';
import { Handheld, type HandMode } from '@scutoons/content/storyboard';
import { WASM_BASE } from './live-deck.ts';
import { cameraKey, heldBefore, keepHeld, store } from './live-cameras.ts';

export class CameraSession {
	/** joining · connected · offline · refused (the core's status) */
	status = $state('joining');
	holding = $state<Holding>({ hold: 'none' });
	cameras = $state<LiveCamera[]>([]);
	/** the conductor's flash for the camera it holds */
	flash = $state<Record<string, unknown> | null>(null);
	mode = $state<HandMode>('none');
	tracking = $state(true);
	xr = $state(false);
	denied = $state(false);

	private client = '';
	private readonly link: CameraLink;
	readonly hand: Handheld;

	/** `relay` the session's URL; `camera` the page's camera (none: the list) */
	constructor(relay: string, camera?: string) {
		const s = store();
		this.link = new CameraLink({
			relay,
			key: cameraKey(s),
			core: loadRelay(WASM_BASE),
			onStatus: (st) => {
				this.status = st.state;
				if (st.client) this.client = st.client;
			},
			onHolding: (h) => {
				this.holding = h;
				keepHeld(s, h.hold === 'held' ? (h.camera ?? null) : null, h.hold === 'held' ? this.client : null);
			},
			onCameras: (c) => (this.cameras = c),
			onFlash: (_, settings) => (this.flash = settings)
		});
		this.hand = new Handheld(this.link.link, () => ([this.mode, this.tracking, this.xr, this.denied] = [this.hand.mode, this.hand.tracking, this.hand.xr, this.hand.error === 'denied']));
		this.hand.probe();
		const was = camera ? heldBefore(s, camera) : null;
		if (camera && was) this.link.resume(camera, was);
	}

	/** the camera as the relay shows it now */
	camera(id: string): LiveCamera | undefined {
		return this.cameras.find((c) => c.id === id);
	}

	claim = (camera: string) => this.link.claim(camera);
	leave = () => this.link.leave();
	recenter = () => this.link.link.recenter();

	close(): void {
		this.hand.close();
		this.link.link.close();
	}
}
