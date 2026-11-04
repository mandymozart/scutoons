// GENERATED from deck/live-core/src/clock.rs and deck-core's tempo by `npm run generate` (cargo run -p live-core --features codegen --bin codecs-ts) – do not edit.
// The core's tempo clock as the page reads it (#321, #370, #372): the state block a clock frame leaves (clock.rs `Slot`s,
// f64 each) read into names – the slots' indices, the flag bits, the clock's sources, and the MIDI messages that reach the
// clock. No decision here: the core decides when to ping, what anchor a leader sends, where the beat is, the picker's
// choice, the MIDI clock out and the strobe's flash. The tables are the Rust side's (Slot, FLAGS, READS, GROUPS).

/** deck-core's `Source::ALL`, in order (the picker's index) */
export const SOURCES = ['internal', 'midi', 'link', 'osc', 'relay'] as const;
export type ClockSource = (typeof SOURCES)[number];

/** the MIDI messages that reach the clock (deck-core's `CLOCK_STATUS`: tick, start, continue, stop, song position) – the port's
 *  filter; notes, controllers and active sensing never reach the core */
export const CLOCK_MESSAGES: ReadonlySet<number> = new Set([0xf8, 0xfa, 0xfb, 0xfc, 0xf2]);

/** the state block's length */
export const STATE_LENGTH = 20;
/** the state block's slots */
export const SLOT = {
	heardMs: 0,
	beat: 1,
	bpm: 2,
	perBar: 3,
	running: 4,
	source: 5,
	flags: 6,
	offsetMs: 7,
	rttMs: 8,
	errorMs: 9,
	sendBpm: 10,
	sendPerBar: 11,
	sendBeat: 12,
	sendAt: 13,
	sendRunning: 14,
	click: 15,
	mode: 16,
	midiOut: 17,
	flashEdgeMs: 18,
	flashMs: 19
} as const;
/** the flags slot's bits */
export const FLAG = {
	locked: 1,
	anchor: 2,
	leading: 4,
	midiLost: 8,
	ping: 16,
	send: 32
} as const;

/** the show clock's anchor (the relay's `tempo`): at relay ms `at` the show was at `beat` */
export interface ClockAnchor {
	bpm: number;
	perBar: number;
	beat: number;
	at: number;
	running: boolean;
}

/** the strobe's flash: its beat edge (the page's ms) and its length (ms) */
export interface ClockFlash {
	atMs: number;
	ms: number;
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
	/** the relay's clock minus this one (ms) */
	offsetMs: number;
	/** the shortest round trip (ms) */
	rttMs: number;
	/** how far off the show's line (ms) */
	errorMs: number;
	click: boolean;
	/** the picker's choice: `ClockMode::ALL`'s index, -1: none of them */
	mode: number;
	/** the MIDI clock out is on */
	midiOut: boolean;
	/** the anchor to send (this deck leads and its line moved) */
	send: ClockAnchor | null;
	/** the strobe's flash on the beat; null: none */
	flash: ClockFlash | null;
	/** deck-core's line (`Controller::line`: the tempo line, the source's state – the native HUD's words) – the host fills it */
	line: string;
}

/** the state block → names */
export function clockOf(s: ArrayLike<number>): ClockState {
	return {
		heardMs: s[SLOT.heardMs],
		beat: s[SLOT.beat],
		bpm: s[SLOT.bpm],
		perBar: s[SLOT.perBar],
		running: s[SLOT.running] !== 0,
		source: SOURCES[s[SLOT.source]] ?? SOURCES[0],
		locked: (s[SLOT.flags] & FLAG.locked) !== 0,
		anchor: (s[SLOT.flags] & FLAG.anchor) !== 0,
		leading: (s[SLOT.flags] & FLAG.leading) !== 0,
		midiLost: (s[SLOT.flags] & FLAG.midiLost) !== 0,
		ping: (s[SLOT.flags] & FLAG.ping) !== 0,
		offsetMs: s[SLOT.offsetMs],
		rttMs: s[SLOT.rttMs],
		errorMs: s[SLOT.errorMs],
		click: s[SLOT.click] !== 0,
		mode: s[SLOT.mode],
		midiOut: s[SLOT.midiOut] !== 0,
		send: s[SLOT.flags] & FLAG.send ? { bpm: s[SLOT.sendBpm], perBar: s[SLOT.sendPerBar], beat: s[SLOT.sendBeat], at: s[SLOT.sendAt], running: s[SLOT.sendRunning] !== 0 } : null,
		flash: s[SLOT.flashMs] > 0 ? { atMs: s[SLOT.flashEdgeMs], ms: s[SLOT.flashMs] } : null,
		line: ''
	};
}
