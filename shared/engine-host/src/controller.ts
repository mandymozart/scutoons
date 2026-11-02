// controller.ts (#298): the deck's browser engine brought up and run by one object – the viewer's /live/deck and the
// creator's editor both use it. It owns what the deck page used to do around `Deck`: the deck (relay-less or joined),
// the being in 3D (`SpaceStage`, deck-space as wasm), the sound (`Deck.start` → live-core: worker, worklet, rings), the
// frame loop, the rig played for a mood, the statistics and a log of what the engine said. No DOM beyond the element the
// 3D is drawn into, no app state, routing or paraglide: the app reaches in through `hooks`.
//
//   const host = createEngineHost({ api, cdn, wasmBase: '/live/', character: 'catpope', rigs: true });  // host.ts
//   await host.mount(element);            // the deck and the being; no sound yet, no gesture needed
//   await host.start();                   // the sound – from a click or a key (the AudioContext needs a gesture)
//   host.playRig(bundle, '', 'happy');    // another rig; host.setMood('happy'); host.stop(); host.dispose()
//
// The engine itself stays in Rust: live-deck's `Deck` (state, relay, sound) and `SpaceStage` (space-web's `rig_play`,
// `core_rig`), live-core's worker and worklet. Nothing of the sound or the being is done in TS here.
import { Deck, FrameStats, SpaceStage, ladderFor, type EngineFiles, type Hand, type RigPlayed } from '@scutoons/live-deck';
import { EngineLog, messageOf } from './log.ts';
import { deckOptions, folder, rigSubject, moodChoices, type EngineHostOptions } from './options.ts';
import { canStart, canStop, emptySnapshot, leadMs, loadPercent, type EnginePhase, type EngineSnapshot } from './status.ts';

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
}

export class EngineHost {
	readonly log = new EngineLog();
	/** the frame statistics (fps, work per frame) */
	readonly frames = new FrameStats();
	deck: Deck | null = null;
	space: SpaceStage | null = null;
	hooks: EngineHooks = {};
	/** the 3D's own line, refreshed every frame */
	hud = '';
	/** play the character's core rig (instead of the fixed patch) whenever the character changes */
	rigs: boolean;
	mood = 'neutral';
	private snap: EngineSnapshot = emptySnapshot();
	private readonly listeners = new Set<(s: EngineSnapshot) => void>();
	private bundle: string | null = null;
	private persona = '';
	private pinned = false;
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

	/** the moods the character's portraits give (`neutral` first) */
	get moods(): string[] {
		return moodChoices(Object.keys(this.deck?.portraits ?? {}));
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
			if (this.rigs) {
				this.applyRig();
				this.space.setBeing();
			}
		} catch (e) {
			this.space = null;
			this.log.push('3d', `no 3D: ${messageOf(e)}`, 'error');
			this.hooks.threeD?.(false, messageOf(e));
		}
		if (!this.alive) return;
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
		this.setPhase('stopped');
		this.sample();
	}

	/** Another character: its portraits, its rig (with `rigs`), the being anew. */
	async pickCharacter(id: string): Promise<void> {
		const deck = this.init();
		await deck.pickCharacter(id);
		if (this.pinned) deck.moods.moods = [this.mood];
		this.applyRig();
		this.space?.setBeing();
	}

	/** Play a rig bundle (JSON: rig + instruments + presets, space-web's `rig_play`) for a persona and a mood – the sliders
	 *  become its knobs, the being its graph, the sound its scene. Throws on a bad bundle; needs the 3D (the rig is
	 *  played by deck-space's wasm). */
	playRig(bundle: string, persona = '', mood = this.mood): RigPlayed {
		const deck = this.deck;
		const space = this.space;
		if (!deck || !space) throw new Error('the engine host has no 3D yet – mount it first (the rig is played by deck-space)');
		const played = space.playRig(bundle, persona, mood);
		this.bundle = bundle;
		this.persona = persona;
		deck.useRig(played);
		space.setBeing();
		this.log.push('rig', `played ${played.character || 'an item'}${mood ? ` (${mood})` : ''}: ${played.knobs.length} knobs, ${played.nodes.length} nodes`);
		this.hooks.rig?.(played, 'play');
		return played;
	}

	/** The core rig of `subject` (`character.catpope`, or just `catpope`) – false when the deck carries none, the fixed
	 *  patch plays then. */
	playCoreRig(subject: string, persona = '', mood = this.mood): boolean {
		const space = this.space;
		if (!this.deck || !space) throw new Error('the engine host has no 3D yet – mount it first');
		const bundle = space.coreRig(rigSubject(subject));
		if (!bundle) {
			this.log.push('rig', `no core rig for ${rigSubject(subject)} – the fixed patch plays`, 'warn');
			this.bundle = null;
			this.deck.useRig(null);
			space.setBeing();
			this.hooks.rig?.(null, 'play');
			return false;
		}
		this.playRig(bundle, persona, mood);
		return true;
	}

	/** Hold the being on `mood` (a portrait's name), or null to let the energy ladder walk it again. The rig follows. */
	setMood(mood: string | null): void {
		const deck = this.deck;
		if (!deck) return;
		this.pinned = mood !== null;
		deck.moods.moods = mood === null ? ladderFor(Object.keys(deck.portraits)) : [mood];
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
		this.deck?.close();
		void this.deck?.stop().catch(() => undefined);
		this.listeners.clear();
		this.snap = { ...this.snap, phase: 'idle' };
	}

	/** the deck's mood changed: the rig for it, then the app */
	private moodChanged(md: string): void {
		this.mood = md;
		this.hooks.mood?.(md);
		const deck = this.deck;
		if (this.bundle && this.space && deck) {
			try {
				deck.moodRig(this.space.playRig(this.bundle, this.persona, md));
				this.hooks.rig?.(deck.rig, 'mood');
			} catch (e) {
				this.log.push('rig', `mood ${md}: ${messageOf(e)}`, 'error');
				this.hooks.spaceStatus?.(messageOf(e));
			}
		}
		this.snap = { ...this.snap, mood: md };
	}

	/** the character's core rig for the deck's character and mood (with `rigs`) */
	private applyRig(): void {
		const deck = this.deck;
		if (!deck || !this.rigs) return;
		this.bundle = null;
		try {
			const space = this.space;
			this.bundle = space ? space.coreRig(rigSubject(deck.character)) : null;
			deck.useRig(this.bundle && space ? space.playRig(this.bundle, this.persona, this.mood) : null);
			this.log.push('rig', this.bundle ? `core rig of ${deck.character}: ${deck.rig?.knobs.length} knobs` : `no core rig for ${deck.character} – the fixed patch plays`);
		} catch (e) {
			this.bundle = null;
			deck.useRig(null);
			this.log.push('rig', messageOf(e), 'error');
			this.hooks.spaceStatus?.(messageOf(e));
		}
		this.hooks.rig?.(deck.rig, 'play');
	}

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
		if (space) {
			try {
				space.frame(now);
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
		this.frames.push(t0, performance.now() - t0);
		this.raf = requestAnimationFrame(this.frame);
	};

	private setPhase(phase: EnginePhase): void {
		if (phase !== this.snap.phase) this.log.push('host', phase);
		this.snap = { ...this.snap, phase };
		this.emit();
	}

	/** the numbers behind the snapshot, every 250 ms */
	private sample(): void {
		const deck = this.deck;
		const core = deck?.core ?? null;
		const s = core?.stats();
		const q = this.space?.quality;
		const glitching = core?.glitching() ?? false;
		if (glitching && !this.wasGlitching) this.log.push('sound', `dropout (${s?.underrunsPerMinute ?? 0} in the last minute)`, 'warn');
		this.wasGlitching = glitching;
		this.snap = {
			...this.snap,
			isolated: globalThis.crossOriginIsolated === true,
			transport: core?.transport ?? null,
			audio: core?.context.state ?? '',
			clock: core?.context.currentTime ?? 0,
			sampleRate: core?.context.sampleRate ?? 0,
			backend: this.space?.backend ?? '',
			fps: this.frames.fps,
			cpu: this.frames.cpu,
			peak: this.frames.peak,
			level: deck?.bands.level ?? 0,
			bass: deck?.bands.bass ?? 0,
			mid: deck?.bands.mid ?? 0,
			high: deck?.bands.high ?? 0,
			reduction: deck?.bands.reduction ?? 0,
			dropouts: s ? s.underrunsPerMinute : null,
			load: s && core ? loadPercent(s.renderPeakUs, s.budgetUs) : 0,
			lead: s && core ? leadMs(s.lead, core.context.sampleRate) : 0,
			glitching,
			quality: q ? `${q.level} ${q.scale.toFixed(2)}×${q.auto ? '' : ' FIX'}` : '',
			hud: this.hud
		};
		this.emit();
	}

	private emit(): void {
		for (const f of this.listeners) f(this.snap);
	}
}
