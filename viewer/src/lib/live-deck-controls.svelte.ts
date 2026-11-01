// The browser deck's controls as page state (#209): the selection (the sticks follow it), MIDI learn and Web MIDI, the
// mappings per deck – a thin layer over @scutoons/live-deck's Hands (which bridges deck-space's controls, wasm). The
// route (/live/deck) and its panel read these runes; the logic stays in Rust (DECISIONS 2026-10-31).
import { Hands, PadSticks, connectMidi, midiSupported, nextSelection, selectionNames, type MidiLink, type Spec } from '@scutoons/live-deck';
import { m } from '$lib/paraglide/messages.js';
import { sticksLine } from '$lib/live-deck.ts';

export class DeckControlsState {
	hands: Hands | null = null;
	/** the being rings what the panel selects (SpaceView.select) */
	onSelect: (name: string) => void = () => {};
	selection = $state('');
	learn = $state(false);
	learning = $state('');
	midiStatus = $state('');
	/** bumped when the mappings change: the knobs re-read their labels */
	private tick = $state(0);
	private midi: MidiLink | null = null;
	/** #228: a Gamepad API pad on the two sticks and the selection (A cycles it) */
	private pad: PadSticks;

	constructor(private specs: Record<string, Spec>) {
		this.pad = new PadSticks(
			{
				stickDown: (side) => this.hands?.stickDown(side),
				stickMove: (side, dx, dy) => this.hands?.stickMove(side, dx, dy),
				stickUp: (side) => this.hands?.stickUp(side)
			},
			() => this.choose(nextSelection(this.selection, selectionNames(Object.keys(this.specs))), true)
		);
	}

	/** #228: once a frame (the page's loop) – the connected pad's sticks and A; nothing without controls */
	poll() {
		if (!this.hands) return;
		this.pad.poll(typeof navigator !== 'undefined' && navigator.getGamepads ? navigator.getGamepads() : []);
	}

	/** #191: the deck's knobs changed (a rig's, or the fixed patch's again) – the sticks and the selection follow */
	useParams(specs: Record<string, Spec>, sticks: [string, string][] | null) {
		this.specs = specs;
		this.hands?.useParams(specs, sticks);
		this.selection = this.hands?.selection ?? '';
		this.tick++;
	}

	/** the panel's line: what the sticks drive now */
	readonly line = $derived.by(() => {
		void this.selection;
		const h = this.hands;
		const of = (p: string) => this.specs[p]?.label;
		return sticksLine(this.selection, of, h?.sticks(0) ?? [], h?.sticks(1) ?? [], { being: m['live_deck.being'](), master: m['live_deck.master']() });
	});

	/** the knob that owns the sticks (its path, "" none) */
	readonly knob = $derived(this.selection.startsWith('knob:') ? this.selection.slice(5) : '');

	/** stick `side`'s [x, y] labels now */
	labels = (side: number) => (this.hands?.sticks(side) ?? []).map((p) => (p ? (this.specs[p]?.label ?? p) : '–'));

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

	private onMidi = (data: Uint8Array) => {
		if (this.hands?.midi(data)?.learned) {
			this.learning = '';
			this.tick++;
		}
	};

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
