// hand.ts (#469, research/HANDHELD-CAMERA.md): the handheld camera's link – a phone (`handheld`) or the storyboard editor
// (its session's conductor) on deck-space's relay client (relay.rs, relay-web's wasm) under client.ts's socket. The core
// decides everything a link decides: the socket (QUIC first, the WebSocket last, the ladder's timing), the lane (a pose is
// a datagram on QUIC), which pose is late or malformed, what is sent again after a rejoin or to a phone that joins late,
// that a phone left, the rung's word. This only hands the page's moves in and the core's words out.
import type { HandPose, HandState, HandView } from '@scutoons/live';
import { RelaySocket, type Platform, type RelayCoreView, type RelayModule, type RelayStatus } from './client.ts';

export { loadRelay } from './client.ts';

/** relay-web's `RelayClient` with a handheld session's moves */
interface HandCore extends RelayCoreView {
	rung(): string;
	hand_pose(t: number, p: Float64Array, q: Float64Array): boolean;
	hand_state(json: string): void;
	hand_record(on: boolean): boolean;
	hand_recenter(): boolean;
	hand_view(json: string): void;
}

export interface HandLinkOptions extends Platform {
	/** the session's relay URL (`handRelay`) */
	relay: string;
	/** the phone, or the editor that showed its code */
	role: 'handheld' | 'conductor';
	/** the API's token for this one session and role (the phone's: its answer to the one-time code) */
	token: string;
	/** relay-web (`loadRelay('/live/')`); the link joins once it is there */
	core: RelayModule | Promise<RelayModule>;
	/** the core's status, the rung it is on ('QUIC' · 'WebSocket' · '' while not joined) and its HUD line */
	onStatus?: (s: RelayStatus, rung: string, line: string) => void;
	/** the editor hears a phone: its pose, state, Record / Stop, Recenter, that it left (it may come back as the same client) */
	onPose?: (client: string, pose: HandPose) => void;
	onState?: (client: string, state: HandState) => void;
	onRecord?: (client: string, on: boolean) => void;
	onRecenter?: (client: string) => void;
	onAway?: (client: string) => void;
	/** a phone hears the editor's view */
	onView?: (view: HandView) => void;
}

/** the session's relay URL: `<live>/live/<session>`, `quic` naming the relay's QUIC door when it is not the relay's own */
export function handRelay(live: string, session: string, quic?: string): string {
	const url = `${live.replace(/\/+$/, '')}/live/${encodeURIComponent(session)}`;
	return quic ? `${url}?quic=${encodeURIComponent(quic)}` : url;
}

export class HandLink {
	private s: (RelaySocket & { core: HandCore }) | null = null;
	private stopped = false;
	/** what the page said before the core was there (the core keeps it from then on) */
	private early: { state?: HandState; view?: HandView } = {};

	constructor(private readonly o: HandLinkOptions) {
		void Promise.resolve(o.core).then(this.begin.bind(this), (e) => o.onStatus?.({ state: 'refused', why: String(e) }, '', ''));
	}

	private begin(m: RelayModule): void {
		if (this.stopped) return;
		const o = this.o;
		const core = new m.RelayClient(JSON.stringify({ relay: o.relay, role: o.role, token: o.token })) as HandCore;
		if (this.early.state) core.hand_state(JSON.stringify(this.early.state));
		if (this.early.view) core.hand_view(JSON.stringify(this.early.view));
		const hooks = { onStatus: (line: string, s: RelayStatus) => o.onStatus?.(s, core.rung(), line), onOther: this.hear.bind(this) };
		this.s = new RelaySocket(core, hooks, o) as RelaySocket & { core: HandCore };
		this.s.start();
	}

	/** #469: the core's hand words (relay.rs `Out::Hand…`) to the page */
	private hear(w: Record<string, never>): void {
		const o = this.o;
		if ('hand_pose' in w) o.onPose?.(w.hand_pose['client'], w.hand_pose['pose']);
		else if ('hand_state' in w) o.onState?.(w.hand_state['client'], w.hand_state['state']);
		else if ('hand_record' in w) o.onRecord?.(w.hand_record['client'], w.hand_record['on']);
		else if ('hand_recenter' in w) o.onRecenter?.(w.hand_recenter['client']);
		else if ('hand_away' in w) o.onAway?.(w.hand_away['client']);
		else if ('hand_view' in w) o.onView?.(w.hand_view);
	}

	/** the phone's pose at `t` (its clock, ms): position (3) and rotation (quaternion, 4) */
	pose(t: number, p: readonly number[], q: readonly number[]): void {
		this.s?.core.hand_pose(t, Float64Array.from(p), Float64Array.from(q));
		this.s?.run();
	}

	/** what the phone tracks (the core sends it again after every rejoin) */
	state(s: HandState): void {
		this.early.state = s;
		this.s?.core.hand_state(JSON.stringify(s));
		this.s?.run();
	}

	record(on: boolean): void {
		this.s?.core.hand_record(on);
		this.s?.run();
	}

	recenter(): void {
		this.s?.core.hand_recenter();
		this.s?.run();
	}

	/** the editor's view for its phones (the core sends it again after every rejoin and to a phone that joins later) */
	view(v: HandView): void {
		this.early.view = v;
		this.s?.core.hand_view(JSON.stringify(v));
		this.s?.run();
	}

	close(): void {
		this.stopped = true;
		this.s?.close();
		this.s = null;
	}
}
