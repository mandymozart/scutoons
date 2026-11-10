// The creator's one door to the handheld camera's relay link (#469; hand-port.ts says why it is handed in): a HandLink
// (@scutoons/live-deck/hand, the core's relay client: QUIC first, the WebSocket last) on the creator's relay (LIVE_URL),
// the core's words as a page's plain calls and back. No decision here – the core makes them all.
import { HandLink, handRelay, loadRelay } from '@scutoons/live-deck/hand';
import { LIVE_QUIC, LIVE_URL } from '$lib/config.ts';
import type { OpenHand } from './hand-port.ts';

/** the relay-web wasm (creator/static/live/, `npm run build:wasm -w shared/live-deck`) */
const WASM_BASE = '/live/';

export const openHand: OpenHand = (role, session, token, end) => {
	const link = new HandLink({
		relay: handRelay(LIVE_URL, session, LIVE_QUIC),
		role,
		token,
		core: loadRelay(WASM_BASE),
		onStatus: (s, rung) => end.status(s.state, rung, s.why ?? ''),
		onState: (_, s) => end.phone?.(s.ua ?? '', s.mode, s.tracking),
		onPose: (_, p) => end.pose?.(p.t, p.p, p.q),
		onRecord: (_, on) => end.record?.(on),
		onRecenter: () => end.recenter?.(),
		onAway: () => end.away?.(),
		onView: (v) => end.view?.(v.shot ?? null, v.on, v.recording, v.seconds)
	});
	return link;
};
