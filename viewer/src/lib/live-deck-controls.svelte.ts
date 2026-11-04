// The browser deck's controls as page state (#209): the selection (the sticks follow it), MIDI learn and Web MIDI, the
// mappings per deck – a thin layer over @scutoons/live-deck's Hands (which bridges deck-space's controls, wasm). The
// route (/live/deck) and its panel read these runes; the logic stays in Rust (DECISIONS 2026-10-31).
import { Hands, connectMidi, midiSupported, pollPad, type MidiLink, type PadSink, type Spec } from '@scutoons/live-deck';
import { m } from '$lib/paraglide/messages.js';

export class DeckControlsState {
	hands: Hands | null = $state.raw(null);
	/** the being rings what the panel selects (SpaceView.select) */
	onSelect: (name: string) => void = () => {};
	selection = $state('');
	learn = $state(false);
	learning = $state('');
	midiStatus = $state('');
	/** bumped when the mappings change: the knobs re-read their labels */
	private tick = $state(0);
	private midi: MidiLink | null = null;
	/** #228: a Gamepad API pad on the two sticks and the selection (A / Y cycle it – the core's rules) */
	private readonly pad: PadSink = {
		stickDown: (side) => this.hands?.stickDown(side),
		stickMove: (side, dx, dy) => this.hands?.stickMove(side, dx, dy),
		stickUp: (side) => this.hands?.stickUp(side),
		selected: () => this.choose(this.hands?.c.selection() ?? '', true)
	};

	/** #228: once a frame (the page's loop) – the connected pad's sticks and buttons; nothing without controls */
	poll() {
		if (!this.hands) return;
		pollPad(this.hands.c, typeof navigator !== 'undefined' && navigator.getGamepads ? navigator.getGamepads() : [], this.pad);
	}

	/** #191: the deck's knobs changed (a rig's, or the deck's own scene's again) – the sticks and the selection follow */
	useParams(specs: Record<string, Spec>, sticks: [string, string][] | null) {
		this.hands?.useParams(specs, sticks);
		this.selection = this.hands?.selection ?? '';
		this.tick++;
	}

	/** the panel's line: what the sticks drive now (the core's) */
	readonly line = $derived.by(() => {
		void this.selection;
		void this.tick;
		return this.hands?.c.selection_line(m['live_deck.being'](), m['live_deck.master']()) ?? '';
	});

	/** the knob that owns the sticks (its path, "" none) */
	readonly knob = $derived.by(() => {
		void this.selection;
		return this.hands?.c.knob() ?? '';
	});

	/** stick `side`'s [x, y] labels now (the core's) */
	labels = (side: number) => [...(this.hands?.c.stick_labels(side) ?? [])];

	/** what drives `path` over MIDI */
	mapped = (path: string) => (this.tick >= 0 && this.hands ? this.hands.mapped(path) : '');

	/** select by name: a tap on the being (`fromPanel` false) or a knob in the panel – the being rings that */
	choose(name: string, fromPanel: boolean) {
		if (!this.hands) return;
		this.hands.select(name);
		this.selection = this.hands.selection;
		if (fromPanel) this.onSelect(name);
	}

	/** a knob tapped in the panel: it owns the sticks; in MIDI learn it waits for a controller */
	pick = (path: string) => {
		if (this.learn && this.hands) {
			this.hands.learn(path);
			this.learning = path;
		}
		this.choose(`knob:${path}`, true);
	};

	toggleLearn = () => {
		this.learn = !this.learn;
		if (this.learn) void this.startMidi();
		else {
			this.hands?.learn('');
			this.learning = '';
		}
	};

	/** the being's mappings (per deck = per character until a preset carries them, #191); MIDI on if it has any */
	load(character: string) {
		if (!this.hands) return;
		this.hands.load(character);
		this.tick++;
		if (this.hands.problems.length) this.midiStatus = m['live_deck.midi_problems']({ lines: this.hands.problems.join(' · ') });
		if (this.hands.text) void this.startMidi();
	}

	/** #321: every message to the deck's clock too (its MIDI clock in reads the clock's among them) */
	onClockMidi: ((data: Uint8Array, timeStamp: number) => void) | null = null;

	private onMidi = (data: Uint8Array, timeStamp: number) => {
		this.onClockMidi?.(data, timeStamp);
		if (this.hands?.midi(data)?.learned) {
			this.learning = '';
			this.tick++;
		}
	};

	/** #321: bytes to every MIDI output at the page's `atMs` (the deck's clock out) – once MIDI is started */
	midiSend(bytes: number[], atMs: number) {
		this.midi?.send(bytes, atMs);
	}

	async startMidi() {
		if (this.midi) return;
		if (!midiSupported()) {
			this.midiStatus = m['live_deck.midi_none']();
			return;
		}
		const line = (l: string[]) => (l.length ? m['live_deck.midi_inputs']({ inputs: l.join(', ') }) : m['live_deck.midi_no_inputs']());
		try {
			this.midi = await connectMidi(this.onMidi, (l) => (this.midiStatus = line(l)));
			this.midiStatus = line(this.midi.inputs());
		} catch (e) {
			this.midiStatus = m['live_deck.midi_denied']({ reason: (e as Error).message ?? String(e) });
		}
	}

	close() {
		this.pad.reset();
		this.midi?.close();
		this.hands?.c.free?.();
	}
}
