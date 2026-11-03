// midi.ts (#209): Web MIDI for the browser deck – the bridge only: the permission, the inputs as they come and go, each
// message handed on as bytes (the mapping and MIDI learn are deck-space's – controls.ts → DeckControls). Chrome on
// Android and desktop have it; Safari / Firefox without it say so (`supported`).

export interface MidiLink {
	/** the inputs' names now */
	inputs(): string[];
	close(): void;
}

/** the minimal Web MIDI shapes this module reads (lib.dom has them; kept narrow for the tests) */
export interface MidiInputLike {
	name?: string | null;
	onmidimessage: ((e: { data: Uint8Array | null; timeStamp?: number }) => void) | null;
}
export interface MidiAccessLike {
	inputs: { forEach(f: (i: MidiInputLike) => void): void };
	onstatechange: ((e: unknown) => void) | null;
}

/** Web MIDI is there (a secure context with navigator.requestMIDIAccess) */
export const midiSupported = (nav: unknown = typeof navigator === 'undefined' ? undefined : navigator) =>
	!!nav && typeof (nav as { requestMIDIAccess?: unknown }).requestMIDIAccess === 'function';

/** Ask for MIDI (the browser's permission prompt) and hand every input's messages to `onMessage`; inputs plugged in
 *  later are picked up (`onChange` says the list changed). Throws when the browser refuses or has no Web MIDI. */
export async function connectMidi(
	/** each message and when it arrived (the event's `timeStamp`, ms on performance.now – #321's MIDI clock in) */
	onMessage: (data: Uint8Array, timeStamp: number) => void,
	onChange?: (inputs: string[]) => void,
	request: () => Promise<MidiAccessLike> = () => (navigator as unknown as { requestMIDIAccess(o?: { sysex: boolean }): Promise<MidiAccessLike> }).requestMIDIAccess({ sysex: false })
): Promise<MidiLink> {
	const access = await request();
	const names = () => {
		const out: string[] = [];
		access.inputs.forEach((i) => out.push(i.name || 'MIDI'));
		return out;
	};
	const wire = () =>
		access.inputs.forEach((i) => {
			i.onmidimessage = (e) => {
				if (e.data && e.data.length) onMessage(e.data, e.timeStamp ?? performance.now());
			};
		});
	wire();
	access.onstatechange = () => {
		wire();
		onChange?.(names());
	};
	return {
		inputs: names,
		close() {
			access.onstatechange = null;
			access.inputs.forEach((i) => (i.onmidimessage = null));
		}
	};
}
