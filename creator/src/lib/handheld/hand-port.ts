// The handheld camera's link as the creator's pages see it (#469): what a page hands in (`HandPort`) and what it hears
// (`HandEnd`), plain values – the relay link behind it is hand-link.ts (@scutoons/live-deck's HandLink on the core's relay
// client), handed in by the page's Svelte side so the storyboard's modules stay outside the deck's bridge (check:core-only).

/** what a page hands to the link (@scutoons/live-deck's HandLink is one) */
export interface HandPort {
	/** the phone's pose at `t` (its clock, ms): position (3), rotation (quaternion, 4) */
	pose(t: number, p: readonly number[], q: readonly number[]): void;
	/** what the phone tracks: 'xr' · 'orientation' · 'none' (the core refuses another word) */
	state(s: { mode: 'xr' | 'orientation' | 'none'; tracking: boolean; ua?: string }): void;
	record(on: boolean): void;
	recenter(): void;
	/** the editor's view for its phones */
	view(v: { shot?: string; on: boolean; recording: boolean; seconds: number }): void;
	close(): void;
}

/** what the link tells a page – the core's words, already checked (a late or malformed pose never comes) */
export interface HandEnd {
	/** the core's link state ('joining' · 'connected' · 'offline' · 'refused'), the rung it is on ('QUIC' · 'WebSocket' ·
	 *  '' while not joined), why it is offline or refused */
	status(state: string, rung: string, why: string): void;
	/** the editor: a phone's state, pose, Record / Stop, Recenter; that it left */
	phone?(ua: string, mode: string, tracking: boolean): void;
	pose?(t: number, p: readonly number[], q: readonly number[]): void;
	record?(on: boolean): void;
	recenter?(): void;
	away?(): void;
	/** a phone: the editor's view */
	view?(shot: string | null, on: boolean, recording: boolean, seconds: number): void;
}

/** a link for one relay session: the phone (`handheld`) or the editor that showed its code (`conductor`), with the API's token */
export type OpenHand = (role: 'handheld' | 'conductor', session: string, token: string, end: HandEnd) => HandPort;
