// clock.ts (#321): the core's tempo clock as the page reads it – the state block a clock frame leaves (deck/live-core
// clock.rs `STATE`, f64) read into names. No decision here: the core decides when to ping, what anchor a leader sends
// and where the beat is; the page moves those bytes and draws (a strobe evaluates the core's line at its own frame).

/** deck-core's `Source::ALL`, in order (the picker's index) */
export const SOURCES = ['internal', 'midi', 'link', 'osc', 'relay'] as const;
export type ClockSource = (typeof SOURCES)[number];

/** the show clock's anchor (the relay's `tempo`): at relay ms `at` the show was at `beat` */
export interface ClockAnchor {
	bpm: number;
	perBar: number;
	beat: number;
	at: number;
	running: boolean;
}

export interface ClockState {
	/** the page's ms (`performance.timeOrigin + now()`) the beat below is heard at */
	heardMs: number;
	beat: number;
	bpm: number;
	perBar: number;
	running: boolean;
	source: ClockSource;
	/** the offset filter has its samples */
	locked: boolean;
	/** an anchor came */
	anchor: boolean;
	/** this deck leads the show clock */
	leading: boolean;
	/** on MIDI clock in: no clock – the last tempo holds */
	midiLost: boolean;
	/** a ping is due (the link sends one) */
	ping: boolean;
	/** the anchor to send (this deck leads and its line moved) */
	send: ClockAnchor | null;
	/** the relay's clock minus this one (ms), the shortest round trip (ms), how far off the show's line (ms) */
	offsetMs: number;
	rttMs: number;
	errorMs: number;
	click: boolean;
}

const LOCKED = 1;
const ANCHOR = 2;
const LEADING = 4;
const MIDI_LOST = 8;
const PING = 16;
const SEND = 32;

/** the state block → names */
export function clockOf(s: ArrayLike<number>): ClockState {
	const flags = s[6];
	return {
		heardMs: s[0],
		beat: s[1],
		bpm: s[2],
		perBar: s[3],
		running: s[4] !== 0,
		source: SOURCES[s[5]] ?? 'internal',
		locked: (flags & LOCKED) !== 0,
		anchor: (flags & ANCHOR) !== 0,
		leading: (flags & LEADING) !== 0,
		midiLost: (flags & MIDI_LOST) !== 0,
		ping: (flags & PING) !== 0,
		send: flags & SEND ? { bpm: s[10], perBar: s[11], beat: s[12], at: s[13], running: s[14] !== 0 } : null,
		offsetMs: s[7],
		rttMs: s[8],
		errorMs: s[9],
		click: s[15] !== 0
	};
}

/** the beat the clock's line says at the page's `ms` (a drawing between two clock frames) */
export const beatAt = (c: ClockState, ms: number): number => (c.running ? c.beat + ((ms - c.heardMs) / 60_000) * c.bpm : c.beat);

/** the page's clock: ms since the epoch, the same in the page, its workers and the worklet's stamps */
export const nowMs = (): number => performance.timeOrigin + performance.now();
