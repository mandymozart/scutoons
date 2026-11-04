// GENERATED from deck/live-core/src/signals.rs by `npm run generate` (cargo run -p live-core --features codegen --bin codecs-ts) – do not edit.
// The scene's `[outputs]` as the browser engine resolves them (#282, #211, #372): device messages – the same three kinds as
// deck/protocol/schema.json's `DeviceMsg` (midi bytes, osc address + typed args, dmx channel values) – which the page
// *shows*; the browser plays no MIDI port or DMX widget. The layout is the Rust side's (signals.rs: its doc comment has the
// byte layout, FIELDS / KINDS / OSC_ARGS the tables this is written from); deck/live-core/fixtures/signals.json holds
// records with their bytes and both sides' tests agree on every one. Records follow each other, each a multiple of 4
// bytes; decoding copies, so the wasm view can be reused.
import { WireError } from './wire.ts';

@@CONSTS@@

/** An OSC argument, typed like protocol/schema.json's `OscArg` (JSON alone cannot tell 2 from 2.0). */
export type OscArg =
@@OSC_TYPES@@;

/** A resolved device message, with the engine clock it was meant for (`at`: ms since the scene loaded; wraps after 49 days). */
export type DeviceSignal =
	/** `port`: the deck's MIDI out port, or the conductor (over the relay) */
	| { kind: '@@KIND_MIDI@@'; port: 'out' | 'conductor'; bytes: number[]; at: number }
	/** `values` from channel `ch` on (1-based) in the universe; `flash`: a strobe flash, sent at once */
	| { kind: '@@KIND_DMX@@'; ch: number; values: number[]; flash: boolean; at: number }
	| { kind: '@@KIND_OSC@@'; address: string; args: OscArg[]; at: number };

/** One protocol's line in the device monitor (#373, deck-core `devices::Row`): worded and judged by the core. */
export interface DeviceRow {
	kind: DeviceSignal['kind'];
	/** messages since the sound started */
	count: number;
	/** the last message in a short line (`CC 74 = 127 · ch 1`) */
	text: string;
	/** 1 on a message, falling to 0 */
	activity: number;
	/** traffic just now – the lamp is lit */
	active: boolean;
}

const utf8 = new TextDecoder();

/** The records of `bytes`, in order; a malformed one throws a WireError with its offset. */
export function* readSignals(bytes: Uint8Array): Generator<DeviceSignal> {
	const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
	let at = 0;
	while (at < bytes.length) {
		if (bytes.length - at < SIGNAL_HEADER) throw new WireError(at, 'a truncated signal header');
		const kind = @@GET_KIND@@;
		const flags = @@GET_FLAGS@@;
		const n = @@GET_N@@;
		const ch = @@GET_CH@@;
		const when = @@GET_AT@@;
		const start = at + SIGNAL_HEADER;
		const end = start + n;
		if (end > bytes.length) throw new WireError(at, 'truncated signal payload');
		if (kind === KIND_MIDI) yield { kind: '@@KIND_MIDI@@', port: flags & FLAG_CONDUCTOR ? 'conductor' : 'out', bytes: [...bytes.subarray(start, end)], at: when };
		else if (kind === KIND_DMX) yield { kind: '@@KIND_DMX@@', ch, values: [...bytes.subarray(start, end)], flash: (flags & FLAG_FLASH) !== 0, at: when };
		else if (kind === KIND_OSC) yield osc(view, bytes, start, end, at, when);
		else throw new WireError(at, 'an unknown signal kind');
		at = end + ((4 - (n % 4)) % 4);
	}
}

function osc(view: DataView, bytes: Uint8Array, start: number, end: number, record: number, when: number): DeviceSignal {
	const bad = () => new WireError(record, 'a malformed osc payload');
	let p = start;
	const need = (k: number) => {
		if (p + k > end) throw bad();
	};
	need(2);
	const len = view.getUint16(p, true);
	p += 2;
	need(len + 1);
	const address = utf8.decode(bytes.subarray(p, p + len));
	p += len;
	const count = view.getUint8(p++);
	const args: OscArg[] = [];
	for (let i = 0; i < count; i++) {
		need(1);
		const tag = view.getUint8(p++);
@@OSC_ARGS@@
		} else throw bad();
	}
	return { kind: '@@KIND_OSC@@', address, args, at: when };
}
