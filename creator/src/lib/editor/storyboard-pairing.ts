// The handheld camera's link between the member's PC and their phone (#126; STUDIO-IN-CREATOR §2.6, server/docs/API.md
// "Handheld pairing"): ONE WebRTC connection with two data channels – `pose` (unordered, no retransmits: a late pose is
// worthless) and `ctl` (reliable: state, record, recenter). The signaling: the PC asks the API for a one-time code (the
// QR to /h/<code>), puts its offer (ICE candidates gathered, public STUN only – no TURN: phone and PC on the same Wi-Fi),
// the phone reads it with the code and puts its answer, the PC polls the answer (that read ends the pairing). The phone
// has no session – the code is its only right. Imported by the phone's barebone page: no Svelte, no paraglide, no three.

/** public STUN only (owner 2026-10-20 plan: no TURN server – the same Wi-Fi) */
export const ICE_SERVERS: RTCIceServer[] = [{ urls: ['stun:stun.l.google.com:19302', 'stun:stun.cloudflare.com:3478'] }];
export const CODE = /^[A-Za-z0-9_-]{22}$/;
/** the PC asks for the answer this often (ms); a pairing lives 10 minutes (the server's Pairings::MINUTES) */
export const POLL_MS = 2000;
/** ICE gathering waits at most this long (ms) – then the candidates found so far go */
export const GATHER_MS = 4000;

/** the phone's page for a code (the creator's barebone route) */
export const phoneUrl = (origin: string, code: string) => `${origin.replace(/\/+$/, '')}/h/${code}`;

// ── Messages ────────────────────────────────────────────────────────────────────────────────────────────

export type Vec3 = [number, number, number];
export type Quat = [number, number, number, number];
/** what the phone tracks: WebXR (position + rotation), the orientation sensor (rotation only), nothing yet */
export type PhoneMode = 'xr' | 'orientation' | '';

/** phone → PC */
export type PhoneMessage =
	| { type: 'hello'; ua: string }
	| { type: 'state'; mode: PhoneMode; tracking: boolean }
	/** the phone's own pose in its world (the PC maps it onto the shot camera): t = the phone's clock, ms */
	| { type: 'pose'; t: number; p: Vec3; q: Quat }
	| { type: 'rec'; on: boolean }
	| { type: 'recenter' };
/** PC → phone */
export type PcMessage = { type: 'state'; shot: string | null; on: boolean; recording: boolean; seconds: number };

const num = (x: unknown): x is number => typeof x === 'number' && Number.isFinite(x);
const nums = (x: unknown, n: number): boolean => Array.isArray(x) && x.length === n && x.every(num);

/** a peer's message, checked field by field (anything else: null – it comes from another device) */
export function decodePhone(text: unknown): PhoneMessage | null {
	const m = parse(text);
	if (!m) return null;
	switch (m.type) {
		case 'hello':
			return { type: 'hello', ua: String(m.ua ?? '').slice(0, 300) };
		case 'state':
			return { type: 'state', mode: m.mode === 'xr' || m.mode === 'orientation' ? m.mode : '', tracking: m.tracking !== false };
		case 'pose':
			return num(m.t) && nums(m.p, 3) && nums(m.q, 4) ? { type: 'pose', t: m.t, p: m.p as Vec3, q: m.q as Quat } : null;
		case 'rec':
			return { type: 'rec', on: m.on === true };
		case 'recenter':
			return { type: 'recenter' };
	}
	return null;
}
export function decodePc(text: unknown): PcMessage | null {
	const m = parse(text);
	if (m?.type !== 'state') return null;
	return { type: 'state', shot: typeof m.shot === 'string' ? m.shot.slice(0, 64) : null, on: m.on === true, recording: m.recording === true, seconds: num(m.seconds) ? m.seconds : 0 };
}
function parse(text: unknown): Record<string, unknown> | null {
	if (typeof text !== 'string' || text.length > 4096) return null;
	try {
		const m = JSON.parse(text);
		return m && typeof m === 'object' && !Array.isArray(m) ? m : null;
	} catch {
		return null;
	}
}

// ── The signaling calls ─────────────────────────────────────────────────────────────────────────────────

export interface Pairing {
	code: string;
	toon: string;
	chapter: number;
	shot: string | null;
	expires_at: string;
}
export interface PairingView extends Pairing {
	offer: string | null;
	answered: boolean;
	/** only for the member who asked for the code */
	answer?: string | null;
}
export interface PairingCalls {
	create(body: { toon: string; chapter: number; shot?: string | null }): Promise<Pairing>;
	offer(code: string, sdp: string): Promise<void>;
	answer(code: string, sdp: string): Promise<void>;
	read(code: string): Promise<PairingView>;
	remove(code: string): Promise<void>;
}

/** the PC: through the creator's signed-in API client */
export function sessionCalls(api: { get<T>(p: string): Promise<T>; post<T>(p: string, b?: unknown): Promise<T>; put<T>(p: string, b?: unknown): Promise<T>; delete<T>(p: string): Promise<T> }): PairingCalls {
	const at = (code: string) => `/storyboard/pairings/${encodeURIComponent(code)}`;
	return {
		create: async (body) => (await api.post<{ pairing: Pairing }>('/storyboard/pairings', body)).pairing,
		offer: async (code, sdp) => void (await api.put(`${at(code)}/offer`, { sdp })),
		answer: async (code, sdp) => void (await api.put(`${at(code)}/answer`, { sdp })),
		read: async (code) => (await api.get<{ pairing: PairingView }>(at(code))).pairing,
		remove: async (code) => void (await api.delete(at(code)))
	};
}

/** an API error with its status (the phone's calls) */
export class PairingError extends Error {
	constructor(
		readonly status: number,
		readonly code: string,
		message: string
	) {
		super(message);
	}
}

/** the phone: plain fetch, no cookie, no session (the code is the right) */
export function publicCalls(base: string, fetchFn: typeof fetch = fetch): PairingCalls {
	const at = (code: string) => `${base.replace(/\/+$/, '')}/storyboard/pairings/${encodeURIComponent(code)}`;
	async function call<T>(method: string, url: string, body?: unknown): Promise<T> {
		const r = await fetchFn(url, { method, credentials: 'omit', cache: 'no-store', headers: body ? { 'Content-Type': 'application/json' } : {}, body: body ? JSON.stringify(body) : undefined });
		const json = (await r.json().catch(() => ({}))) as Record<string, unknown>;
		if (!r.ok) throw new PairingError(r.status, String(json.error ?? r.status), String(json.message ?? r.statusText));
		return json as T;
	}
	const no = () => Promise.reject(new Error('The phone has no session.'));
	return {
		create: no,
		remove: no,
		offer: async (code, sdp) => void (await call('PUT', `${at(code)}/offer`, { sdp })),
		answer: async (code, sdp) => void (await call('PUT', `${at(code)}/answer`, { sdp })),
		read: async (code) => (await call<{ pairing: PairingView }>('GET', at(code))).pairing
	};
}

/** the PC waits for the phone's answer (polled every POLL_MS; aborted → null; the code ran out → the API's 404 throws) */
export async function waitAnswer(calls: Pick<PairingCalls, 'read'>, code: string, opts: { signal?: AbortSignal; every?: number; sleep?: (ms: number) => Promise<void> } = {}): Promise<string | null> {
	const sleep = opts.sleep ?? ((ms: number) => new Promise<void>((go) => setTimeout(go, ms)));
	while (!opts.signal?.aborted) {
		const view = await calls.read(code);
		if (view.answer) return view.answer;
		await sleep(opts.every ?? POLL_MS);
	}
	return null;
}

// ── The connection ──────────────────────────────────────────────────────────────────────────────────────

/** the part of an RTCPeerConnection waited on here (a fake in the tests) */
export interface Gathering {
	iceGatheringState: RTCIceGatheringState;
	localDescription: { sdp: string } | null;
	addEventListener(type: 'icegatheringstatechange', fn: () => void): void;
	removeEventListener(type: 'icegatheringstatechange', fn: () => void): void;
}

/** the local description once the ICE candidates are gathered (no trickle: one offer / answer each way) */
export function gathered(pc: Gathering, ms = GATHER_MS): Promise<string> {
	return new Promise((resolve, reject) => {
		const done = () => {
			pc.removeEventListener('icegatheringstatechange', check);
			clearTimeout(timer);
			if (pc.localDescription?.sdp) resolve(pc.localDescription.sdp);
			else reject(new Error('No session description.'));
		};
		const check = () => {
			if (pc.iceGatheringState === 'complete') done();
		};
		const timer = setTimeout(done, ms);
		pc.addEventListener('icegatheringstatechange', check);
		check();
	});
}

export interface Channels {
	ctl: RTCDataChannel;
	pose: RTCDataChannel;
}

/** the PC's side: the two channels + the offer, put under the code */
export async function makeOffer(pc: RTCPeerConnection, calls: Pick<PairingCalls, 'offer'>, code: string): Promise<Channels> {
	const ctl = pc.createDataChannel('ctl');
	const pose = pc.createDataChannel('pose', { ordered: false, maxRetransmits: 0 });
	await pc.setLocalDescription(await pc.createOffer());
	await calls.offer(code, await gathered(pc as unknown as Gathering));
	return { ctl, pose };
}

/** the PC: the phone's answer in */
export const takeAnswer = (pc: RTCPeerConnection, sdp: string) => pc.setRemoteDescription({ type: 'answer', sdp });

/** the phone's side: the PC's offer read, the answer put; → the pairing (its shot) and the channels once both are open */
export async function makeAnswer(pc: RTCPeerConnection, calls: Pick<PairingCalls, 'read' | 'answer'>, code: string): Promise<{ view: PairingView; channels: Promise<Channels> }> {
	const view = await calls.read(code);
	if (!view.offer) throw new PairingError(409, 'no_offer', 'The PC has not put its offer yet.');
	if (view.answered) throw new PairingError(409, 'answer_exists', 'This code is used already.');
	const channels = new Promise<Channels>((resolve) => {
		const got: Partial<Channels> = {};
		pc.addEventListener('datachannel', (e) => {
			const ch = e.channel;
			if (ch.label === 'ctl' || ch.label === 'pose') got[ch.label] = ch;
			if (got.ctl && got.pose) resolve(got as Channels);
		});
	});
	await pc.setRemoteDescription({ type: 'offer', sdp: view.offer });
	await pc.setLocalDescription(await pc.createAnswer());
	await calls.answer(code, await gathered(pc as unknown as Gathering));
	return { view, channels };
}

/** a message out (only on an open channel) */
export function send(ch: RTCDataChannel | null | undefined, m: PhoneMessage | PcMessage): void {
	if (ch?.readyState === 'open') ch.send(JSON.stringify(m));
}
