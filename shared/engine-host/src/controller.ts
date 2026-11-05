// controller.ts (#298): the deck's browser engine brought up and run by one object – the viewer's /live/deck and the
// creator's editor both use it. It owns what the deck page used to do around `Deck`: the deck (relay-less or joined),
// the being in 3D (`SpaceStage`, deck-space as wasm), the sound (`Deck.start` → live-core: worker, worklet, rings), the
// frame loop, the rig played for a mood, the statistics and a log of what the engine said. No DOM beyond the element the
// 3D is drawn into, no app state, routing or paraglide: the app reaches in through `hooks`.
//
//   const host = createEngineHost({ api, cdn, wasmBase: '/live/', character: 'catpope', rigs: true });  // host.ts
//   await host.mount(element);            // the deck and the being; no sound yet, no gesture needed
//   await host.start();                   // the sound – from a click or a key (the AudioContext needs a gesture)
//   host.playRig(bundle);                 // another rig; host.setMood('happy'); host.stop(); host.dispose()
//
// The engine itself stays in Rust: live-deck's `Deck` (relay, sound) on deck-space's deck (space-web `DeckState`: the
// mood, what plays, the rig's lifecycle – #367, #368), `SpaceStage` (the being), live-core's worker and worklet. Nothing
// of the sound, the rig or the being is decided in TS here.
import { METER_ZONES, type DeviceRow, type DeviceSignal, type OutputReading } from '@scutoons/live-core';
import { Deck, SpaceStage, loadMonitor, loadPush, type EngineFiles, type Hand, type MonitorReport, type MonitorView, type RigPlayed, type RigTurn } from '@scutoons/live-deck';
import { EngineLog, formatLine, messageOf } from './log.ts';
import { deckOptions, folder, type EngineHostOptions } from './options.ts';
import { canStart, canStop, emptySnapshot, type EnginePhase, type EngineSnapshot } from './status.ts';
import { deviceInfo, engineReport } from './report.ts';

/** How an app listens in. Every hook is optional; each runs inside the host's own try/catch. */
export interface EngineHooks {
	/** the 3D started (its view is there): load the quality presets, bind the controls to it */
	space?: (space: SpaceStage) => void;
	/** the 3D is not there (it did not start, or a frame threw and it stopped); the deck and the sound go on */
	threeD?: (ok: boolean, reason: string) => void;
	/** the 3D's status line (`3D: loading …`, `3D webgpu`), or why a rig failed */
	spaceStatus?: (s: string) => void;
	/** after the deck's own tick, once per animation frame (the controls' poll) */
	tick?: (now: number) => void;
	/** a tap on the being: '' (empty space), 'being' or `knob:node.port` */
	tap?: (name: string) => void;
	/** a rig was played (`play`: new knobs, sticks, graph) or only replayed for another mood (`mood`: values) */
	rig?: (played: RigPlayed | null, why: 'play' | 'mood') => void;
	/** the mood changed (the deck's ladder, or `setMood`) – the rig is already replayed for it */
	mood?: (mood: string) => void;
	/** a parameter turned: the page's controls, the being's buttons, a mood's preset */
	value?: (path: string, value: number, hand: Hand) => void;
	/** #282: a device message the scene's `[outputs]` resolved to – MIDI bytes, an OSC address + typed args, DMX channel
	 *  values (the protocol's `DeviceMsg` kinds) with the engine clock it was meant for. Shown, not played: the browser
	 *  has no MIDI port or DMX widget. A few a second at most; the core's device monitor already keeps the last of each kind. */
	signal?: (msg: DeviceSignal) => void;
}

export class EngineHost {
	readonly log = new EngineLog();
	/** #364: deck-core's monitor (space-web, no GPU) – fed each frame, the worker's audio window, the dropouts; its report
	 *  is `snapshot().stats`. null until it loaded (or when the wasm is missing) */
	perf: MonitorView | null = null;
	deck: Deck | null = null;
	space: SpaceStage | null = null;
	hooks: EngineHooks = {};
	/** #282, #373: the output peaks (0..1) the core holds and lets fall (deck-core meters), as the analysis frame carries
	 *  them – a view on the newest frame, no allocation: read it from a requestAnimationFrame loop (a canvas meter) without
	 *  the snapshot / subscribe path. Its length is the number of outputs (0 while no sound runs); read the property each
	 *  frame rather than keeping it. */
	get peaks(): Float32Array {
		return this.deck?.bands.outputs ?? NO_PEAKS;
	}
	/** the 3D's own line, refreshed every frame */
	hud = '';
	/** play the character's core rig (instead of the fixed patch) whenever the character changes */
	rigs: boolean;
	/** the mood the deck's core shows ('' before it said) */
	mood = '';
	private snap: EngineSnapshot = emptySnapshot();
	private readonly listeners = new Set<(s: EngineSnapshot) => void>();
	private alive = false;
	private raf = 0;
	private timer: ReturnType<typeof setInterval> | undefined;
	private tickThrew = false;
	private wasGlitching = false;

	constructor(
		private readonly o: EngineHostOptions,
		private readonly files: (wasmBase: string) => EngineFiles
	) {
		this.rigs = o.rigs ?? false;
	}

	get phase(): EnginePhase {
		return this.snap.phase;
	}

	/** the engine's state as of the last sample (every 250 ms while mounted) */
	snapshot(): EngineSnapshot {
		return this.snap;
	}

	/** `f` gets a snapshot now and after every sample and phase change; returns the unsubscribe */
	subscribe(f: (s: EngineSnapshot) => void): () => void {
		this.listeners.add(f);
		f(this.snap);
		return () => this.listeners.delete(f);
	}

	/** the moods a picker offers (the deck's core: the calm one first, then the character's portraits) */
	get moods(): string[] {
		return this.deck?.state?.choices() ?? [];
	}

	/** The deck alone, synchronously – an app that sets itself up around the deck (controls, a relay) before the 3D
	 *  starts uses `init` then `attach`; everyone else `mount`. */
	init(): Deck {
		if (this.deck) return this.deck;
		this.alive = true;
		this.setPhase('mounting');
		const deck = new Deck(
			deckOptions(this.o, (s) => {
				this.log.push('deck', s);
				this.o.onStatus?.(s);
			})
		);
		deck.onValue = (p, v, hand) => this.hooks.value?.(p, v, hand);
		deck.onMood = (md) => this.moodChanged(md);
		deck.onTurn = this.turned;
		deck.onSignal = (s) => this.signalled(s);
		this.deck = deck;
		return deck;
	}

	/** the deck, its character's portraits, and the being drawn into `el` – the sound waits for `start` */
	async mount(el: HTMLElement): Promise<void> {
		const deck = this.init();
		await deck.pickCharacter(this.o.character);
		this.log.push('host', `character ${this.o.character}: ${Object.keys(deck.portraits).length} portraits`);
		await this.attach(el);
	}

	/** The being in 3D (wasm, WebGPU else WebGL2) into `el`, the rig if `rigs`, and the frame loop. A 3D that does not
	 *  start is no failure: the deck (and its sound) goes on, `hooks.threeD` says so. */
	async attach(el: HTMLElement): Promise<void> {
		const deck = this.init();
		const o = this.o;
		try {
			this.perf ??= await loadMonitor(folder(o.wasmBase));
		} catch (e) {
			this.log.push('host', `no monitor: ${messageOf(e)}`, 'warn');
		}
		try {
			// #371: the answers to the conductor's pushes are the core's (deck-space's push machine)
			if (deck.sync) deck.sync.useCore(await loadPush(folder(o.wasmBase), o.api, folder(o.cdn)));
		} catch (e) {
			this.log.push('host', `no push answers: ${messageOf(e)}`, 'warn');
		}
		try {
			this.space = await SpaceStage.start({
				host: el,
				base: folder(o.wasmBase),
				deck,
				api: o.api,
				cdn: folder(o.cdn),
				set: o.set,
				prefer: o.prefer,
				maxScale: o.maxScale,
				onStatus: (s) => {
					this.log.push('3d', s);
					this.hooks.spaceStatus?.(s);
				}
			});
			this.hooks.space?.(this.space);
			if (this.rigs) this.playCoreRig(deck.character);
		} catch (e) {
			this.space = null;
			this.log.push('3d', `no 3D: ${messageOf(e)}`, 'error');
			this.hooks.threeD?.(false, messageOf(e));
		}
		if (!this.alive) {
			// disposed while the 3D started: nobody stops it later
			this.space?.stop();
			this.space = null;
			return;
		}
		this.raf = requestAnimationFrame(this.frame);
		this.timer = setInterval(() => this.sample(), 250);
		if (this.snap.phase === 'mounting') this.setPhase('ready');
		this.sample();
	}

	/** Start the sound. Needs a user gesture (the AudioContext); false = it did not start (`snapshot().error` says why). */
	async start(): Promise<boolean> {
		const deck = this.deck;
		if (!deck || !canStart(this.snap.phase)) return this.snap.phase === 'running';
		this.setPhase('starting');
		try {
			await deck.start({ ...this.files(folder(this.o.wasmBase)), transport: this.o.transport });
			const core = deck.core;
			this.log.push('sound', `running: ${core?.transport} transport, ${core?.context.sampleRate} Hz, crossOriginIsolated ${globalThis.crossOriginIsolated === true}`);
			this.snap = { ...this.snap, error: '' };
			core?.audioTo(this.perf);
			this.perf?.reset_devices();
			this.setPhase('running');
			return true;
		} catch (e) {
			this.snap = { ...this.snap, error: messageOf(e) };
			this.log.push('sound', messageOf(e), 'error');
			this.setPhase('error');
			return false;
		}
	}

	/** Stop the sound (the context closes). The being, the deck's values and the rig stay; `start` plays again. */
	async stop(): Promise<void> {
		if (!canStop(this.snap.phase) || !this.deck) return;
		try {
			await this.deck.stop();
		} catch (e) {
			this.log.push('sound', `stop: ${messageOf(e)}`, 'warn');
		}
		this.log.push('sound', 'stopped');
		this.perf?.silent();
		this.setPhase('stopped');
		this.sample();
	}

	/** Another character: its portraits, its rig (with `rigs`), the being anew. */
	async pickCharacter(id: string): Promise<void> {
		const deck = this.init();
		await deck.pickCharacter(id);
		if (this.rigs) this.playCoreRig(id);
		else this.space?.setBeing();
	}

	/** Play a rig bundle (JSON: rig + instruments + presets) for a persona at the deck's mood – the core plays it (and
	 *  replays it on every mood): the sliders become its knobs, the being its graph, the sound its scene. Throws on a bad
	 *  bundle (what played stays) or before the deck's core loaded (`mount`). */
	playRig(bundle: string, persona = ''): RigPlayed {
		return this.init().play(bundle, persona).played;
	}

	/** The core rig of `subject` (`character.catpope`, or just `catpope`) – false when the deck carries none (or before its
	 *  core loaded): the deck's own scene plays then. */
	playCoreRig(subject: string, persona = ''): boolean {
		const deck = this.init();
		return deck.state ? deck.play(subject, persona, true).rig !== null : false;
	}

	/** Hold the being on `mood` (a portrait's name), or null to let the core's energy ladder walk it again. The rig follows. */
	setMood(mood: string | null): void {
		const deck = this.deck;
		if (!deck) return;
		deck.state?.pin(mood ?? '');
		if (mood !== null) this.log.push('host', `mood held: ${mood}`);
	}

	/** Everything off: the frame loop, the 3D, the sound, the relay. The host is not used again. */
	dispose(): void {
		this.alive = false;
		cancelAnimationFrame(this.raf);
		clearInterval(this.timer);
		this.timer = undefined;
		this.space?.stop();
		this.space = null;
		this.deck?.core?.audioTo(null);
		this.deck?.close();
		void this.deck?.stop().catch(() => undefined);
		this.listeners.clear();
		this.snap = { ...this.snap, phase: 'idle' };
	}

	/** a device message from the engine: deck-core's device monitor words it and lights its lamp, then the app (its throw is
	 *  the app's, not the engine's) */
	private signalled(s: DeviceSignal): void {
		this.perf?.signal(performance.now(), JSON.stringify(s));
		try {
			this.hooks.signal?.(s);
		} catch (e) {
			this.log.push('host', `signal hook threw: ${messageOf(e)}`, 'error');
		}
	}

	/** the deck's mood changed (the core replayed the rig for it already): the app */
	private moodChanged(md: string): void {
		this.mood = md;
		this.hooks.mood?.(md);
		this.snap = { ...this.snap, mood: md };
	}

	/** the core's turn was applied (a scene played, the rig replayed for a mood): the log, the being anew, the app */
	private turned = (t: RigTurn): void => {
		if (t.note) this.log.push('rig', t.note, t.failed ? 'error' : 'info');
		if (t.failed) this.hooks.spaceStatus?.(t.note);
		if (t.reload) this.space?.setBeing();
		this.hooks.rig?.(this.deck?.rig ?? null, t.why);
	};

	/** One animation frame: the deck's tick, the 3D's frame, the statistics. A throw out of the deck's tick (or the pad's
	 *  poll) must not end the loop – the 3D and the pointer die with it (#274); reported once. */
	private frame = (): void => {
		const deck = this.deck;
		if (!this.alive || !deck) return;
		const t0 = performance.now();
		const now = t0 / 1000;
		try {
			deck.tick(now);
			this.hooks.tick?.(now);
		} catch (e) {
			if (!this.tickThrew) this.log.push('host', `a frame threw: ${messageOf(e)}`, 'error');
			this.tickThrew = true;
		}
		const space = this.space;
		let drawn = false;
		if (space) {
			try {
				drawn = space.frame(now);
				this.hud = space.view.hud_line();
				const tap = space.view.take_tap();
				if (tap !== undefined) this.hooks.tap?.(tap);
			} catch (e) {
				this.log.push('3d', `the 3D stopped: ${messageOf(e)}`, 'error');
				space.stop();
				this.space = null;
				this.hooks.threeD?.(false, messageOf(e));
			}
		}
		this.perf?.frame(t0, performance.now() - t0, drawn);
		this.raf = requestAnimationFrame(this.frame);
	};

	private setPhase(phase: EnginePhase): void {
		if (phase !== this.snap.phase) this.log.push('host', phase);
		this.snap = { ...this.snap, phase };
		this.emit();
	}

	/** the numbers behind the snapshot, every 250 ms – the monitor's report (#364: deck-core judges and words them) */
	private sample(): void {
		const deck = this.deck;
		const core = deck?.core ?? null;
		const q = this.space?.quality;
		const now = performance.now();
		const perf = this.perf;
		let stats: MonitorReport | null = null;
		if (perf) {
			perf.target(q?.fps ?? 0);
			if (core) perf.underruns(now, core.stats().underruns);
			stats = JSON.parse(perf.report(now)) as MonitorReport;
		}
		if (stats?.alert && !this.wasGlitching) this.log.push('sound', stats.alert_line, 'warn');
		this.wasGlitching = !!stats?.alert;
		this.snap = {
			...this.snap,
			isolated: globalThis.crossOriginIsolated === true,
			transport: core?.transport ?? null,
			audio: core?.context.state ?? '',
			clock: core?.context.currentTime ?? 0,
			sampleRate: core?.context.sampleRate ?? 0,
			backend: this.space?.backend ?? '',
			level: deck?.bands.level ?? 0,
			bass: deck?.bands.bass ?? 0,
			mid: deck?.bands.mid ?? 0,
			high: deck?.bands.high ?? 0,
			reduction: deck?.bands.reduction ?? 0,
			outputs: readings(deck?.bands),
			signals: perf ? (JSON.parse(perf.devices(now)) as DeviceRow[]) : [],
			stats,
			quality: q ? `${q.level} ${q.scale.toFixed(2)}×${q.auto ? '' : ' FIX'}` : '',
			hud: this.hud
		};
		this.emit();
	}

	private emit(): void {
		for (const f of this.listeners) f(this.snap);
	}

	/** #356: the sound's measurement as one text to copy – the device, the context, the block times, the first seconds,
	 *  the log's last lines (report.ts) */
	report(): string {
		this.sample();
		const core = this.deck?.core ?? null;
		return engineReport({
			...deviceInfo(core?.context),
			snapshot: this.snap,
			channels: core?.channels ?? 0,
			series: this.perf?.series() ?? [],
			log: this.log.lines.slice(-20).map(formatLine),
			clock: core?.clockReport ?? ''
		});
	}
}

const NO_PEAKS = new Float32Array(0);

/** the newest analysis frame's meter readings – the core's, copied out for the snapshot */
function readings(bands: { outputs: Float32Array; positions: Float32Array; zones: Float32Array } | undefined): OutputReading[] {
	if (!bands) return [];
	return Array.from(bands.outputs, (level, i) => ({ level, position: bands.positions[i] ?? 0, zone: METER_ZONES[bands.zones[i] ?? 0] ?? 'ok' }));
}
