// deck.ts (#197, #198): the browser deck's state – one for the lab page and the viewer's /live/deck. The parameter
// values (the pads, the sliders, the being's 3D buttons all turn these), the mood ladder, the relay link and its values
// stream, and the sound: live-core – the native deck's engine (#320) – plays the whole scene (its own, #360, or the rig's);
// the page only sends the values and reads back what is heard. No DOM: the page draws it; `tick` runs it once per
// animation frame. With a CDN given it answers the conductor's scene sync (#195, sync.ts). The tempo clock (#321) is the
// core's: the link's pongs and anchors and Web MIDI's clock go in, the core's pings and a leader's anchors go out.
import type { DeckGraph, Tempo } from '@scutoons/live';
import { LiveCore, beatAt, nowMs, SOURCES, type Analysis, type ClockSource, type ClockState, type DeviceKind, type DeviceSignal } from '@scutoons/live-core';
import type { DeckSynced } from '@scutoons/live';
import { DeckLink, ValueStream, type WebSocketLike } from './client.ts';
import { BrowserSync } from './sync.ts';
import { ladderFor, Moods, type Spec } from './scene.ts';
import { knobTarget, rigDiff, rigGraph, rigParams, rigStreamed } from './rig.ts';
import type { RigPlayed } from './space.ts';

/** #321: the MIDI clock's messages: tick, start, continue, stop, song position */
// core-only-ok: #321 the port's filter (as the native port's in tempo/midi.rs) – which bytes reach the core, no parsing
const CLOCK_MESSAGES = new Set([0xf8, 0xfa, 0xfb, 0xfc, 0xf2]);

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
	/** #195: each answer to the conductor's push (the page's status) */
	onSynced?: (r: DeckSynced) => void;
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
	/** the character's rig as played for the current mood (deck-space via space-web `rig_play`); null = the deck's own scene */
	rig: RigPlayed | null = null;
	/** #360: the deck's own scene, the X1's cathedral as deck-space plays it (space-web `deck_scene`); null until it came */
	own: RigPlayed | null = null;
	/** the corner sticks' (x, y) pairs, left then right: the played scene's (#191) */
	sticks: [string, string][] | null = null;
	character: string;
	portraits: Portraits = {};
	readonly moods = new Moods(['neutral'], 1.5);
	/** #321: the core's tempo clock as its last frame left it (null before the sound started) */
	clock: ClockState | null = null;
	/** #321: the show clock's last anchor the link brought (the core gets it when the sound starts) */
	private anchor: { t: Tempo; arrived: number } | null = null;
	/** #321: the picker's choice (kept over a stop / start) */
	private choice: { source: ClockSource; lead: boolean } | null = null;
	/** #321: the MIDI clock out is on (kept for the next `start`), and where its bytes go (Web MIDI's outputs) */
	private out = false;
	onMidiOut: ((bytes: number[], atMs: number) => void) | null = null;
	mood = 'neutral';
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
	private last = -1;
	/** the page's clock at the first tick (s) */
	private t0 = -1;

	constructor(private readonly o: DeckOptions) {
		this.character = o.character;
		this.sync = o.cdn
			? new BrowserSync({
					api: o.api,
					cdn: o.cdn,
					report: (r) => {
						this.link?.synced(r);
						o.onSynced?.(r);
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

	/** #321: the clock's picker – a source (the show clock = `relay`) and whether this deck leads */
	chooseClock(source: ClockSource, lead = false): void {
		this.choice = { source, lead };
		this.core?.clockOp({ op: 'choose', source: SOURCES.indexOf(source), lead });
	}

	/** #321: MIDI clock out on / off – deck-core's schedule (the native deck's), its bytes to `onMidiOut`; none while the
	 *  deck follows MIDI clock in (the core's rule) */
	clockOut(on: boolean): void {
		this.out = on;
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

	/** #321: the beat the clock says at the page's ms (the strobe, the being's breath); NaN before the sound started */
	beatAt(ms = nowMs()): number {
		return this.clock ? beatAt(this.clock, ms) : NaN;
	}

	/** the scene that plays: the rig, else the deck's own */
	get played(): RigPlayed | null {
		return this.rig ?? this.own;
	}

	/** the being on the wire now: the played scene's graph, the character, its ladder first, then every mood it has */
	graph(): DeckGraph {
		const have = Object.keys(this.portraits);
		const moods = [...this.moods.moods, ...have.filter((m) => !this.moods.moods.includes(m))];
		const g = this.played ? rigGraph(this.played, moods) : { nodes: [], links: [], size: 0.5, moods, expose: [] };
		// the deck's own scene wears the deck's character; a rig is its character's
		return this.rig ? g : { ...g, character: this.character };
	}

	/** Play a rig instead of the deck's own scene (null = back to it): its knobs become the sliders, its graph the being
	 *  on the wire, its lowered scene the sound (live-core reloads when it runs). */
	useRig(rig: RigPlayed | null): void {
		this.rig = rig;
		const p = this.played;
		this.sticks = p ? p.sticks.map(([x, y]) => [x, y] as [string, string]) : null;
		this.params = p ? rigParams(p) : {};
		for (const k of Object.keys(this.value)) delete this.value[k];
		for (const [k, s] of Object.entries(this.params)) this.value[k] = s.value;
		this.stream = new ValueStream(p ? rigStreamed(p) : {});
		this.link?.setGraph(this.graph());
		if (this.core) void this.loadEngine(this.core);
	}

	/** #360: the deck's own scene (space-web `deck_scene`) – it plays whenever no rig does */
	useOwn(p: RigPlayed): void {
		this.own = p;
		if (!this.rig) this.useRig(null);
	}

	/** what the controls drive: the played scene's knobs */
	get controlSpecs(): Record<string, Spec> {
		return this.params;
	}

	/** The rig played for the new mood: the values that moved go out (the sliders follow), a new shape reloads. */
	moodRig(p: RigPlayed): void {
		if (!this.rig) return this.useRig(p);
		const d = rigDiff(this.rig, p);
		if (d.reload) return this.useRig(p);
		this.rig = p;
		for (const [path, v] of d.values) {
			if (path in this.value) this.setParam(path, v, 'mood');
			else this.send(path, v);
		}
	}

	/** the paths the deck streams (and the 3D being reads) */
	get paths(): string[] {
		return this.stream.paths;
	}

	/** Another character: its portraits from scene-content, its mood ladder, the being on the wire anew. */
	async pickCharacter(id: string): Promise<void> {
		this.character = id;
		this.portraits = await portraitsOf(this.o.api, id);
		this.moods.moods = ladderFor(Object.keys(this.portraits));
		this.link?.setGraph(this.graph());
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

	/** a knob's value to the engine: its path is the scene's (a rig's node the browser does not carry yet takes none) */
	private send(path: string, v: number): void {
		const t = this.played && knobTarget(this.played, path);
		if (t) this.engineSet(t.engine, [v]);
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

	/** One frame at `now` (s, performance.now() / 1000): the mood, the stream. */
	tick(now: number): void {
		if (this.t0 < 0) this.t0 = this.last = now;
		const dt = Math.min(0.1, Math.max(0, now - this.last));
		this.last = now;
		const m = this.moods.update(now, dt, this.bands.level, this.touch);
		if (m !== this.mood) {
			this.mood = m;
			this.onMood?.(m);
		}
		const changes = this.stream.sample(now, (p) => this.read(p), this.mood);
		if (changes) this.link?.send(changes);
	}

	/** Start the sound (needs a user gesture): live-core with the deck's own scene or the rig. */
	async start(files: EngineFiles): Promise<void> {
		const c = await LiveCore.start({ worker: files.worker, wasm: files.wasm, worklet: files.worklet, transport: files.transport ?? 'auto', device: files.device ?? deviceKind() });
		c.onAnalysis((a) => (this.bands = a));
		c.onSignal((s) => this.onSignal?.(s));
		// #321: the core says when to ping and what a leading deck sends; the ladder reads its tempo
		c.onClock((s) => {
			this.clock = s;
			this.moods.bpm = s.bpm;
			if (s.ping) this.link?.sendPing();
			if (s.send) this.link?.tempo({ bpm: s.send.bpm, per_bar: s.send.perBar, beat: s.send.beat, running: s.send.running, at: s.send.at });
		});
		// #321: the core's MIDI clock out, message by message, for Web MIDI (the page's `onMidiOut` writes the bytes)
		c.onMidiOut((m) => {
			for (let i = 0; i + 3 < m.length; i += 4) this.onMidiOut?.([m[i + 1], m[i + 2], m[i + 3]].filter((b) => !Number.isNaN(b)), m[i]);
		});
		this.core = c;
		if (this.choice) this.chooseClock(this.choice.source, this.choice.lead);
		if (this.out) c.clockOp({ op: 'out', on: true });
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
		for (const p of Object.keys(this.controlSpecs)) this.setParam(p, this.value[p]);
	}

	close() {
		this.link?.close();
	}
}

/** an analysis of nothing (before the sound starts, after it stops) */
const silence = (): Analysis => ({ level: 0, bass: 0, mid: 0, high: 0, spectrum: new Float32Array(0), reduction: 0, outputs: new Float32Array(0), values: new Float32Array(0) });

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
