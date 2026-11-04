// deck.ts (#197, #198): the browser deck's state – one for the lab page and the viewer's /live/deck. The parameter
// values (the pads, the sliders, the being's 3D buttons all turn these), the relay link and its values stream, and the
// sound: live-core – the native deck's engine (#320) – plays the whole scene (its own, #360, or the rig's); the page only
// sends the values and reads back what is heard. The mood and what plays are the core's (#367, #368: deck-space's deck,
// space-web `DeckState`): the deck applies its turns. No DOM: the page draws it; `tick` runs it once per
// animation frame. With a CDN given it answers the conductor's scene sync (#195, sync.ts). The tempo clock (#321) is the
// core's: the link's pongs and anchors and Web MIDI's clock go in, the core's pings and a leader's anchors go out.
import type { DeckGraph, Tempo } from '@scutoons/live';
import { CLOCK_MESSAGES, FRAME, LiveCore, analysisOf, beatAt, nowMs, type Analysis, type ClockState, type DeviceKind, type DeviceSignal } from '@scutoons/live-core';
import type { DeckSynced } from '@scutoons/live';
import { DeckLink, ValueStream, type WebSocketLike } from './client.ts';
import { BrowserSync } from './sync.ts';
import type { Spec } from './scene.ts';
import type { MoodStep, RigPlayed, RigSlider, RigTurn } from './rig.ts';
import type { DeckStateView } from './space.ts';

/** a character's portraits by mood (scene-content's `characters.<id>.portraits`) */
export type Portraits = Record<string, { path: string }>;

export interface DeckOptions {
	/** `ws://host:port/live/<code>`; none = the deck plays and draws alone (no relay yet) */
	relay?: string;
	character: string;
	/** the deck's name at the relay (a member's name, else a default) */
	nick: string;
	/** the relay session token (later, #180 / #186 – see client.ts) */
	token?: string;
	/** the library API (scene-content) */
	api: string;
	/** the CDN root (ends with /): with it the deck answers the conductor's scene sync (#195) */
	cdn?: string;
	/** #195: each answer to the conductor's push and the core's PUSH line for it (the page's status, #371) */
	onSynced?: (r: DeckSynced, line: string) => void;
	onStatus?: (s: string) => void;
	socket?: new (url: string) => WebSocketLike;
}

/** where the sound engine's files are (the host bundles or serves them) */
export interface EngineFiles {
	worker: Worker;
	wasm: URL | string;
	worklet: URL | string;
	transport?: 'auto' | 'shared' | 'transfer';
	/** #365: the device class the core's lead is chosen for (default: `deviceKind()`) */
	device?: DeviceKind;
}

/** who turned a value: the page's controls, the being's 3D buttons, or a mood's preset (the page moves its sliders
 *  for the last two) */
export type Hand = 'page' | 'being' | 'mood';

export class Deck {
	/** every parameter's value (the sliders': the played scene's knobs) */
	readonly value: Record<string, number> = {};
	/** the sliders: the knobs of the scene that plays – the deck's own or the rig's (#191) */
	params: Record<string, Spec> = {};
	/** what plays as the core played it (the rig for the mood now, else the deck's own scene); null until the wasm came */
	played: RigPlayed | null = null;
	/** the character's rig as played for the mood now; null = the deck's own scene (#360, the X1's cathedral) plays */
	rig: RigPlayed | null = null;
	/** the corner sticks' (x, y) pairs, left then right: the played scene's (#191) */
	sticks: [string, string][] | null = null;
	character: string;
	portraits: Portraits = {};
	/** #367, #368: the deck's state in the core (space-web `DeckState`: the mood ladder, what plays); null until the wasm came */
	state: DeckStateView | null = null;
	/** #321: the core's tempo clock as its last frame left it (null before the sound started) */
	clock: ClockState | null = null;
	/** #321: the show clock's last anchor the link brought (the core gets it when the sound starts) */
	private anchor: { t: Tempo; arrived: number } | null = null;
	/** #370: the core's last word on its choice and MIDI clock out (its last frame's), for the next `start`'s core */
	private resume: Pick<ClockState, 'mode' | 'midiOut'> | null = null;
	/** #321: where the MIDI clock out's bytes go (Web MIDI's outputs) */
	onMidiOut: ((bytes: number[], atMs: number) => void) | null = null;
	/** the mood the core's ladder shows ('' until it said) */
	mood = '';
	bands: Analysis = silence();
	/** a finger on a pad (0 / 1): lifts the energy a little */
	touch = 0;
	readonly link: DeckLink | null;
	/** #195: the answers to the conductor's pushes (null without a CDN) */
	readonly sync: BrowserSync | null;
	/** #282: a device message the scene's `[outputs]` resolved to (MIDI / OSC / DMX – shown by the page, never played here) */
	onSignal: ((s: DeviceSignal) => void) | null = null;
	stream = new ValueStream({});
	core: LiveCore | null = null;
	/** the scene live-core plays is the deck's current one: false while a load is in flight (a character switch, a mood's
	 *  new shape) – no `core.set` then, its table is the old scene's and a path it lacks throws (the S22 crash on a switch) */
	private engineReady = false;
	/** each load's number: a load that another overtook is ignored when it lands */
	private loadGen = 0;
	/** every value change, with whose hand (the page's controls follow the being's) */
	onValue: ((path: string, value: number, hand: Hand) => void) | null = null;
	onMood: ((mood: string) => void) | null = null;
	/** #368: the core's turn was applied (a scene played, or the rig replayed for a mood) */
	onTurn: ((t: RigTurn) => void) | null = null;
	private last = -1;
	/** the page's clock at the first tick (s) */
	private t0 = -1;

	constructor(private readonly o: DeckOptions) {
		this.character = o.character;
		this.sync = o.cdn
			? new BrowserSync({
					report: (r, line) => {
						this.link?.synced(r);
						o.onSynced?.(r, line);
					}
				})
			: null;
		const onSync = this.sync ? (s: Parameters<BrowserSync['push']>[0]) => void this.sync!.push(s) : undefined;
		this.link = o.relay
			? new DeckLink({
					relay: o.relay,
					graph: this.graph(),
					nick: o.nick.slice(0, 24),
					token: o.token,
					onStatus: o.onStatus,
					socket: o.socket,
					onSync,
					onPong: (t0, t1, t2) => this.core?.clockOp({ op: 'pong', t0, t1, t2 }),
					onTempo: (t, arrived) => {
						this.anchor = { t, arrived };
						this.giveAnchor();
					},
					onJoined: (now) => this.core?.clockOp({ op: 'joined', now })
				})
			: null;
	}

	/** #321: the show clock's anchor into the core (when it runs) */
	private giveAnchor(): void {
		const a = this.anchor;
		if (!a || a.t.at === undefined) return;
		this.core?.clockOp({ op: 'tempo', bpm: a.t.bpm, perBar: a.t.per_bar, beat: a.t.beat, at: a.t.at, running: a.t.running, arrived: a.arrived });
	}

	/** #370: the clock's picker – a `ClockMode::ALL` index (`state.clock_modes()`; the clock frame says it: `clock.mode`) */
	clockMode(mode: number): void {
		this.core?.clockOp({ op: 'mode', mode });
	}

	/** #321: MIDI clock out on / off – deck-core's schedule (the native deck's), its bytes to `onMidiOut`; none while the
	 *  deck follows MIDI clock in (the core's rule); the clock frame says it (`clock.midiOut`, #370) */
	clockOut(on: boolean): void {
		this.core?.clockOp({ op: 'out', on });
	}

	/** #321: a key of the clock: tap, faster, slower, start / stop, the click */
	clockKey(key: 'tap' | 'faster' | 'slower' | 'startstop' | 'click'): void {
		this.core?.clockOp({ op: 'key', key: ['tap', 'faster', 'slower', 'startstop', 'click'].indexOf(key), at: nowMs() });
	}

	/** #321: a Web MIDI message (the browser's `timeStamp`, ms on the page's `performance.now`): the core's clock reads the
	 *  clock messages among them */
	midiIn(data: ArrayLike<number>, timeStamp: number): void {
		// only the clock's messages (tick, start, continue, stop, song position) – as the native port: notes, CC and
		// active sensing never reach the worker
		if (CLOCK_MESSAGES.has(data[0])) this.core?.clockOp({ op: 'midi', bytes: [data[0], data[1] ?? 0, data[2] ?? 0], at: performance.timeOrigin + timeStamp });
	}

	/** #321: the beat the clock says at the page's ms (the being's breath); NaN before the sound started */
	beatAt(ms = nowMs()): number {
		return this.clock ? beatAt(this.clock, ms) : NaN;
	}

	/** #367, #368: the core's deck state – the character's portraits go in, what it plays (its own scene) is loaded */
	useState(s: DeckStateView): void {
		this.state = s;
		s.set_portraits(Object.keys(this.portraits));
		this.mood = s.mood();
		this.apply(JSON.parse(s.now()) as RigTurn);
	}

	/** #368: a rig bundle (JSON) for a persona at the mood now ('' = the deck's own scene; throws on a bad one), or with
	 *  `core` the core rig of a character / subject (else the deck's own scene – the turn's note says so) */
	play(what: string, persona = '', core = false): RigTurn {
		const s = this.state;
		if (!s) throw new Error('the deck has no core yet (space-web was not loaded)');
		return this.apply(JSON.parse(core ? s.play_core(what, persona) : s.play(what, persona)) as RigTurn);
	}

	/** #368: the core's turn – a scene to load whole (sliders, sticks, stream, wire, sound) or the values a mood moved */
	private apply(t: RigTurn): RigTurn {
		this.played = t.played;
		this.rig = t.rig ? t.played : null;
		if (t.reload) {
			this.sticks = t.played.sticks.map(([x, y]) => [x, y] as [string, string]);
			this.params = Object.fromEntries((JSON.parse(this.state?.sliders('') ?? '[]') as RigSlider[]).map((s) => [s.path, s]));
			for (const k of Object.keys(this.value)) delete this.value[k];
			for (const [k, s] of Object.entries(this.params)) this.value[k] = s.value;
			this.stream = new ValueStream(Object.fromEntries(t.played.streamed));
			this.link?.setGraph(this.graph());
			if (this.core) void this.loadEngine(this.core);
		} else for (const [path, v] of t.values) path in this.value ? this.setParam(path, v, 'mood') : this.send(path, v);
		this.onTurn?.(t);
		return t;
	}

	/** #367: the portrait `mood` shows (the core's fallback), null when there is none */
	face(mood: string): { path: string } | null {
		return this.portraits[this.state?.face(mood) ?? ''] ?? null;
	}

	/** the being on the wire now (the core's: the played scene, its character – the deck's for its own scene –, the moods) */
	graph(): DeckGraph {
		return this.state ? (JSON.parse(this.state.graph(this.character)) as DeckGraph) : { nodes: [], links: [] };
	}

	/** the paths the deck streams (and the 3D being reads) */
	get paths(): string[] {
		return this.stream.paths;
	}

	/** Another character: its portraits from scene-content, its mood ladder (the core's), the being on the wire anew. */
	async pickCharacter(id: string): Promise<void> {
		this.character = id;
		this.portraits = await portraitsOf(this.o.api, id);
		this.state?.set_portraits(Object.keys(this.portraits));
		this.link?.setGraph(this.graph());
	}

	/** #369: where knob `path`'s value sits on its range, 0..1 (the core's – 0 before it came or for no knob) */
	norm(path: string): number {
		return this.state?.norm(path, this.value[path] ?? NaN) ?? 0;
	}

	/** a parameter from the page's controls or the being's buttons: the knob turns its input's base in the engine (#188 –
	 *  a link on it keeps flowing on top) */
	setParam(path: string, v: number, hand: Hand = 'page') {
		if (!(path in this.value) || !Number.isFinite(v)) return;
		this.value[path] = v;
		this.onValue?.(path, v, hand);
		this.send(path, v);
	}

	/** a value into live-core – only once its scene is the deck's, and never throwing out of the deck (a path the
	 *  scene lacks is nothing) */
	private engineSet(param: string, values: number[]): void {
		if (!this.core || !this.engineReady) return;
		try {
			this.core.set(param, values, { mode: 'base' });
		} catch {
			/* a param the engine's scene doesn't have */
		}
	}

	/** a knob's value to the engine, where the core says it goes (a node the browser does not carry yet takes none) */
	private send(path: string, v: number): void {
		const t = this.state?.target(path);
		if (t) this.engineSet(t, [v]);
	}

	/** a streamed path's value now: what the engine played in the heard block (an input, a control output, an audio
	 *  output's peak), else the page's own value of a knob */
	read(p: string): number {
		const heard = this.engineReady ? (this.core?.value(p) ?? NaN) : NaN;
		return Number.isFinite(heard) ? heard : (this.value[p] ?? NaN);
	}

	/** the page's clock since the first tick (s) – the being's beat */
	get time(): number {
		return this.t0 < 0 ? 0 : this.last - this.t0;
	}

	/** One frame at `now` (s, performance.now() / 1000): the mood (the core's ladder: the level, the clock, the hand) and
	 *  the rig the core replayed for it, the stream. */
	tick(now: number): void {
		if (this.t0 < 0) this.t0 = this.last = now;
		const dt = now - this.last;
		this.last = now;
		const said = this.state?.step(this.bands.level, this.clock?.bpm ?? 0, this.clock?.running ?? false, this.touch, dt);
		if (said) {
			const s = JSON.parse(said) as MoodStep;
			if (s.turn) this.apply(s.turn);
			if (s.error) this.o.onStatus?.(s.error);
			this.mood = s.mood;
			this.onMood?.(s.mood);
		}
		const changes = this.stream.sample(now, (p) => this.read(p), this.mood);
		if (changes) this.link?.send(changes);
	}

	/** Start the sound (needs a user gesture): live-core with the deck's own scene or the rig. */
	async start(files: EngineFiles): Promise<void> {
		const c = await LiveCore.start({ worker: files.worker, wasm: files.wasm, worklet: files.worklet, transport: files.transport ?? 'auto', device: files.device ?? deviceKind() });
		c.onAnalysis((a) => (this.bands = a));
		c.onSignal((s) => this.onSignal?.(s));
		// #321: the core says when to ping and what a leading deck sends; the ladder reads its tempo (`tick`)
		c.onClock((s) => {
			this.clock = s;
			if (s.ping) this.link?.sendPing();
			if (s.send) this.link?.tempo({ bpm: s.send.bpm, per_bar: s.send.perBar, beat: s.send.beat, running: s.send.running, at: s.send.at });
		});
		// #321: the core's MIDI clock out, message by message, for Web MIDI (the page's `onMidiOut` writes the bytes)
		c.onMidiOut((m) => {
			for (let i = 0; i + 3 < m.length; i += 4) this.onMidiOut?.([m[i + 1], m[i + 2], m[i + 3]].filter((b) => !Number.isNaN(b)), m[i]);
		});
		this.core = c;
		if (this.resume && this.resume.mode >= 0) this.clockMode(this.resume.mode);
		if (this.resume?.midiOut) this.clockOut(true);
		this.giveAnchor();
		await this.loadEngine(c);
	}

	/** Stop the sound (live-core; the AudioContext closes). The deck keeps its values and rig and can `start` again
	 *  (#298). */
	async stop(): Promise<void> {
		const c = this.core;
		this.core = null;
		this.engineReady = false;
		this.loadGen++;
		this.bands = silence();
		if (this.clock) this.resume = { mode: this.clock.mode, midiOut: this.clock.midiOut };
		this.clock = null;
		await c?.stop();
	}

	/** (Re)load the sound: live-core's scene on the deck's clock, then every slider's value. The engine is quiet from
	 *  here until the scene landed; a newer load (a second switch before this one landed) wins. A rig's nodes the
	 *  browser does not carry yet (its `engine.page`) are not played – the status says which. No scene yet: `useOwn` loads. */
	private async loadEngine(c: LiveCore): Promise<void> {
		const gen = ++this.loadGen;
		this.engineReady = false;
		const rig = this.rig;
		if (!this.played) return;
		try {
			await c.load(this.played.engine.scene);
		} catch (e) {
			if (gen === this.loadGen) this.o.onStatus?.(`sound: ${(e as Error).message ?? e}`);
			return;
		}
		if (gen !== this.loadGen) return;
		this.engineReady = true;
		if (rig?.engine.page.length) this.o.onStatus?.(`sound: not in the browser yet – ${rig.engine.page.map((n) => `${n.id} (${n.kind})`).join(', ')}`);
		for (const p of Object.keys(this.params)) this.setParam(p, this.value[p]);
	}

	close() {
		this.link?.close();
	}
}

/** an analysis of nothing (before the sound starts, after it stops) */
/** an analysis frame of silence: all zeros, no outputs, no values (the codec reads it like any frame) */
const silence = (): Analysis => analysisOf(new Float32Array(FRAME));

/** #365: the device class the core's lead is chosen for – a coarse pointer (a touch screen) is a phone; the numbers
 *  are the core's (deck-core `lead`) */
export function deviceKind(coarse = typeof matchMedia === 'function' && matchMedia('(pointer: coarse)').matches): DeviceKind {
	return coarse ? 'phone' : 'desktop';
}

/** scene-content's portraits of `id` (none offline) */
export async function portraitsOf(api: string, id: string): Promise<Portraits> {
	try {
		const r = await fetch(`${api}/library/scene-content?characters=${encodeURIComponent(id)}`);
		return ((await r.json()) as { characters?: Record<string, { portraits?: Portraits }> }).characters?.[id]?.portraits ?? {};
	} catch {
		return {};
	}
}

/** the characters a deck can wear (`/library/characters`), the current one first if the list lacks it */
export async function charactersOf(api: string, current: string): Promise<{ id: string; name?: Record<string, string> }[]> {
	let list: { id: string; name?: Record<string, string> }[] = [];
	try {
		list = ((await (await fetch(`${api}/library/characters`)).json()) as { characters?: typeof list }).characters ?? [];
	} catch {
		/* offline: the current one only */
	}
	if (!list.some((c) => c.id === current)) list.unshift({ id: current });
	return list;
}
