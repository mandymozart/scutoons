// The phone's side of the storyboard's handheld camera (research/HANDHELD-CAMERA.md, STUDIO-IN-CREATOR §2.6): the
// one-time code from the QR → the API's claim (the relay session and a handheld token for it) → the relay session through
// the link the page hands in (hand-link.ts); the phone's motion is the one handheld (@scutoons/content/storyboard
// `Handheld`), its poses go to the PC, which maps them onto the shot camera, smooths, shows and records. Record / Stop and
// Recenter go to the PC as messages. No session, no paraglide, no three: the barebone page.
import { Handheld, type HandMode } from '@scutoons/content/storyboard';
import { API_URL } from '$lib/config.ts';
import { PairingError, publicCalls, CODE, type PairingCalls } from '$lib/editor/storyboard-pairing.ts';
import type { HandPort, OpenHand } from './hand-port.ts';

export type PhoneStatus = 'nocode' | 'connecting' | 'connected' | 'expired' | 'failed' | 'closed';
export type PhoneMode = HandMode;

export class PhoneSession {
	status = $state<PhoneStatus>('connecting');
	shot = $state<string | null>(null);
	/** the PC says the handheld camera is on for its shot */
	on = $state(true);
	recording = $state(false);
	seconds = $state(0);
	mode = $state<PhoneMode>('none');
	tracking = $state(true);
	/** WebXR immersive-ar is there */
	xr = $state(false);
	error = $state('');
	/** the rung of the ladder the phone is on (the core's word: QUIC · WebSocket; '' while it joins) */
	rung = $state('');

	private link: HandPort | null = null;
	private closed = false;
	/** the phone's motion: its poses go to the link once there is one */
	readonly hand = new Handheld(
		{ pose: (t, p, q) => this.link?.pose(t, p, q), state: (s) => this.link?.state(s) },
		() => ([this.mode, this.tracking, this.xr, this.error] = [this.hand.mode, this.hand.tracking, this.hand.xr, this.hand.error || (this.hand.mode !== 'none' ? '' : this.error)])
	);

	constructor(
		readonly code: string,
		private readonly open: OpenHand,
		private readonly calls: PairingCalls = publicCalls(API_URL)
	) {
		if (!CODE.test(code)) this.status = 'nocode';
	}

	async connect(): Promise<void> {
		if (this.status === 'nocode') return;
		this.hand.probe();
		try {
			const claim = await this.calls.claim(this.code);
			if (this.closed) return;
			this.shot = claim.shot;
			this.link = this.open('handheld', claim.session, claim.token, {
				status: (state, rung, why) => {
					this.rung = rung;
					if (state === 'connected') this.status = 'connected';
					else if (state === 'refused') [this.status, this.error] = ['expired', why];
					else if (this.status === 'connected') this.status = 'connecting';
				},
				view: (shot, on, recording, seconds) => {
					[this.shot, this.on, this.recording, this.seconds] = [shot, on, recording, seconds];
				}
			});
			this.hand.sendState();
		} catch (e) {
			this.status = e instanceof PairingError && (e.status === 404 || e.status === 409) ? 'expired' : 'failed';
			this.error = e instanceof Error ? e.message : String(e);
		}
	}

	record = () => this.link?.record(!this.recording);
	recenter = () => this.link?.recenter();

	close(): void {
		this.closed = true;
		this.hand.close();
		this.link?.close();
		this.link = null;
	}
}
