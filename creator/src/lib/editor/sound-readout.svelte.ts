// What Sound design's panels read from the deck playing the rig (the Monitor's engine host): the core's wiring of what it
// plays (Nodes), its outputs and limiter (Out), its knobs (Knobs), its moods and the mood asked for (the Monitor's chip).
// One per workspace page, alive while the page is – a panel folded, moved or taken full keeps the deck running.
import { emptySnapshot, type EngineHost, type EngineSnapshot } from '@scutoons/engine-host';
import type { GraphWiring } from '@scutoons/rig-designer';
import { inputMeters, knobColumns, outputMeters, sameColumns, stripStatus, type KnobColumn, type KnobLike } from './signal-strip.ts';

export class SoundReadout {
	host = $state<EngineHost | null>(null);
	/** the core's graph of what plays, anew after every rig turn */
	wiring = $state<GraphWiring>();
	snap = $state<EngineSnapshot>(emptySnapshot());
	moods = $state<string[]>([]);
	/** 'auto' or a mood asked for */
	mood = $state('auto');
	columns = $state<KnobColumn[]>([]);
	/** the limiter's gain reduction as 0..1 */
	reduction = $state(0);
	private rigKnobs: KnobLike[] = [];
	private locale: () => string;

	constructor(locale: () => string) {
		this.locale = locale;
	}

	/** the host's hooks and snapshots into this readout; the returned function lets go */
	attach(h: EngineHost): () => void {
		this.wiring = h.deck?.wiring();
		h.hooks = {
			rig: (played) => ((this.rigKnobs = played?.knobs ?? []), (this.wiring = h.deck?.wiring())),
			// once per animation frame: the knobs' live values (the being's buttons, a mood's preset) and the limiter
			tick: () => {
				const d = h.deck;
				if (!d) return;
				const next = knobColumns(this.rigKnobs, d, this.locale());
				if (!sameColumns(next, this.columns)) this.columns = next;
				const gr = d.bands.limiterBar;
				if (Math.abs(gr - this.reduction) >= 0.01) this.reduction = gr;
			}
		};
		return h.subscribe((s) => (this.snap = s));
	}

	pickMood(value: string): void {
		this.mood = value;
		this.host?.setMood(value === 'auto' ? null : value);
	}

	/** one reading per audio output (none while no sound runs), the core's, held and placed on the dB scale (#373) */
	get outputs() {
		return outputMeters(this.snap.outputs);
	}
	get stats() {
		return stripStatus(this.snap);
	}
	get signals() {
		return this.snap.signals;
	}
	/** the engine reports no audio inputs yet (their sources are still to be decided) */
	readonly inputs = inputMeters([]);
}
