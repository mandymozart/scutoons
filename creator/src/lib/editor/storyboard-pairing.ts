// The handheld camera's pairing (#126, #469; research/HANDHELD-CAMERA.md, STUDIO-IN-CREATOR §2.6, server/docs/API.md
// "Handheld pairing"): the PC asks the API for a one-time code (the QR to /h/<code>) and gets a relay session with its own
// conductor token; the phone claims the code once and gets a handheld token for the same session. Both then join that
// session on the live road through @scutoons/live-deck's HandLink (the core picks the socket: QUIC first, the WebSocket
// last). The phone has no session – the code is its only right. Imported by the phone's barebone page: no Svelte, no
// paraglide, no three.

export const CODE = /^[A-Za-z0-9_-]{22}$/;

/** the phone's page for a code (the creator's barebone route) */
export const phoneUrl = (origin: string, code: string) => `${origin.replace(/\/+$/, '')}/h/${code}`;

export interface Pairing {
	code: string;
	toon: string;
	chapter: number;
	shot: string | null;
	expires_at: string;
	/** the relay session the phone and the PC join */
	session: string;
	/** the PC's conductor token for it */
	token: string;
}
/** the phone's claim: the shot it films, the session and its handheld token */
export interface Claim {
	shot: string | null;
	session: string;
	token: string;
}
export interface PairingCalls {
	create(body: { toon: string; chapter: number; shot?: string | null }): Promise<Pairing>;
	claim(code: string): Promise<Claim>;
	remove(code: string): Promise<void>;
}

/** the PC: through the creator's signed-in API client */
export function sessionCalls(api: { post<T>(p: string, b?: unknown): Promise<T>; delete<T>(p: string): Promise<T> }): PairingCalls {
	const at = (code: string) => `/storyboard/pairings/${encodeURIComponent(code)}`;
	return {
		create: async (body) => (await api.post<{ pairing: Pairing }>('/storyboard/pairings', body)).pairing,
		claim: () => Promise.reject(new Error('The PC does not claim its own code.')),
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
	const no = () => Promise.reject(new Error('The phone has no session.'));
	return {
		create: no,
		remove: no,
		claim: async (code) => {
			const r = await fetchFn(`${at(code)}/claim`, { method: 'POST', credentials: 'omit', cache: 'no-store' });
			const json = (await r.json().catch(() => ({}))) as Record<string, unknown>;
			if (!r.ok) throw new PairingError(r.status, String(json.error ?? r.status), String(json.message ?? r.statusText));
			return json.claim as Claim;
		}
	};
}
