// deck.ts (#197, #198): the browser deck's state – one for the lab page and the viewer's /live/deck. The parameter
// values (the pads, the sliders, the being's 3D buttons all turn these), the melody, the LFO, the incense noise, the
// mood ladder, the relay link and its values stream, and the sound (live-core + a Web Audio noise). No DOM: the page
// draws it; `tick` runs it once per animation frame. With a CDN given it answers the conductor's scene sync (#195,
// sync.ts).
import type { DeckGraph } from '@scutoons/live';
import { LiveCore, type Analysis } from '@scutoons/live-core';
import type { DeckSynced } from '@scutoons/live';
import { DeckLink, ValueStream, type WebSocketLike } from './client.ts';
import { BrowserSync } from './sync.ts';
import { BASE_HZ, deckGraph, ENGINE_TOML, ladderFor, MELODY, Moods, PARAMS, SPREAD_OCTAVES, STREAMED, toPos, type Spec } from './scene.ts';
import { knobTarget, rigDiff, rigGraph, rigParams, rigStreamed } from './rig.ts';
import type { RigPlayed } from './space.ts';
import { pinkNoise, type PageNoise } from './noise.ts';

/** the tempo the deck's ladder and the being's breath read (cathedral.toml's) */
export const BPM = 84;

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
	lead?: number;
}

/** who turned a value: the page's controls, the being's 3D buttons, or a mood's preset (the page moves its sliders
 *  for the last two) */
export type Hand = 'page' | 'being' | 'mood';

const clamp01 = (x: number) => Math.min(1, Math.max(0, x));

export class Deck {
	/** every parameter's value (the sliders' – PARAMS or the rig's knobs – + MELODY) */
	readonly value: Record<string, number> = Object.fromEntries(Object.entries({ ...PARAMS, ...MELODY }).map(([k, s]) => [k, s.value]));
	/** the sliders: the fixed patch's PARAMS, or the rig's knobs (#191) */
	params: Record<string, Spec> = PARAMS;
	/** the character's rig as played for the current mood (deck-space via space-web `rig_play`); null = the fixed patch */
	rig: RigPlayed | null = null;
	/** the corner sticks' (x, y) pairs, left then right: the rig's (#191), null = the fixed patch's defaults */
	sticks: [string, string][] | null = null;
	character: string;
	portraits: Portraits = {};
	readonly moods = new Moods(['neutral'], 1.5, BPM);
	mood = 'neutral';
	bands: Analysis = { level: 0, bass: 0, mid: 0, high: 0, spectrum: new Float32Array(0), reduction: 0 };
	/** a finger on a pad (0 / 1): lifts the energy a little */
	touch = 0;
	readonly link: DeckLink | null;
	/** #195: the answers to the conductor's pushes (null without a CDN) */
	readonly sync: BrowserSync | null;
	stream = new ValueStream(STREAMED);
	core: LiveCore | null = null;
	/** the scene live-core plays is the deck's current one: false while a load is in flight (a character switch, a mood's
	 *  new shape) – no `core.set` then, its table is the old scene's and a path it lacks throws (the S22 crash on a switch) */
	private engineReady = false;
	/** each load's number: a load that another overtook is ignored when it lands */
	private loadGen = 0;
	/** every value change, with whose hand (the page's controls follow the being's) */
	onValue: ((path: string, value: number, hand: Hand) => void) | null = null;
	onMood: ((mood: string) => void) | null = null;
	/** the fixed patch's incense, or a rig's noise nodes by id */
	private noises = new Map<string, PageNoise>();
	private gate = 0;
	private swayPhase = 0;
	private sway = 0;
	private pitch = BASE_HZ;
	private target = BASE_HZ;
	private glideFrom = BASE_HZ;
	private glideAt = 0;
	private lastStep = -1;
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
		this.link = o.relay ? new DeckLink({ relay: o.relay, graph: this.graph(), nick: o.nick.slice(0, 24), token: o.token, onStatus: o.onStatus, socket: o.socket, onSync }) : null;
	}

	/** the being on the wire now: the character, its ladder first, then every mood it has */
	graph(): DeckGraph {
		const have = Object.keys(this.portraits);
		const moods = [...this.moods.moods, ...have.filter((m) => !this.moods.moods.includes(m))];
		return this.rig ? rigGraph(this.rig, moods) : deckGraph(this.character, moods);
	}

	/** Play a rig instead of the fixed patch (null = back to it): its knobs become the sliders, its graph the being on
	 *  the wire, its lowered scene the sound (live-core reloads when it runs). */
	useRig(p: RigPlayed | null): void {
		this.rig = p;
		this.sticks = p ? p.sticks.map(([x, y]) => [x, y] as [string, string]) : null;
		this.params = p ? rigParams(p) : PARAMS;
		for (const k of Object.keys(this.value)) if (!(k in MELODY)) delete this.value[k];
		for (const [k, s] of Object.entries(this.params)) this.value[k] = s.value;
		this.stream = new ValueStream(p ? rigStreamed(p) : STREAMED);
		this.link?.setGraph(this.graph());
		if (this.core) void this.loadEngine(this.core);
	}

	/** what the controls drive: a rig's knobs, or the fixed patch's sliders and its page melody */
	get controlSpecs(): Record<string, Spec> {
		return this.rig ? this.params : { ...PARAMS, ...MELODY };
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

	/** a parameter from the page's controls or the being's buttons */
	setParam(path: string, v: number, hand: Hand = 'page') {
		if (!(path in this.value) || !Number.isFinite(v)) return;
		this.value[path] = v;
		this.onValue?.(path, v, hand);
		if (this.rig) return this.send(path, v);
		if (path === 'voice.ratio' || path === 'voice.index') this.engineSet(path, [v]);
		else if (path === 'voice.gain') this.engineSet('vgain.b', [v]);
		else if (path === 'drone.freq' || path === 'drone.gain') {
			this.engineSet('drone', [this.value['drone.freq'], this.value['drone.gain']]);
			this.engineSet('fifth', [this.value['drone.freq'] * 1.5, this.value['drone.gain'] * 0.3]);
		} else if (path === 'incense.gain') this.noiseGain('incense', v);
	}

	/** a value into live-core – only once its scene is the deck's, and never throwing out of the deck (a path the
	 *  scene lacks, e.g. a kind live-core skips, is nothing) */
	private engineSet(param: string, values: number[]): void {
		if (!this.core || !this.engineReady) return;
		try {
			this.core.set(param, values);
		} catch {
			/* a param the engine's scene doesn't have */
		}
	}

	/** a rig's value to where it plays: live-core (through a strip's alias) or a noise the page plays */
	private send(path: string, v: number): void {
		const t = this.rig && knobTarget(this.rig, path);
		if (!t) return;
		if ('engine' in t) this.engineSet(t.engine, [v]);
		else if (t.port === 'gain') this.noiseGain(t.page, v);
		else if (t.port === 'cutoff') this.noises.get(t.page)?.filter.frequency.setTargetAtTime(v, this.noises.get(t.page)!.filter.context.currentTime, 0.03);
	}

	private noiseGain(id: string, v: number): void {
		const n = this.noises.get(id);
		n?.gain.gain.setTargetAtTime(v * n.strip, n.gain.context.currentTime, 0.05);
	}

	/** the incense's cutoff now: its slider swayed by the LFO */
	cutoff(): number {
		return Math.min(12000, this.value['incense.cutoff'] * 2 ** (this.sway * 1.5));
	}

	/** a streamed path's value now (the hoses' sources are the page's own: its melody, its levels, its LFO) */
	read(p: string): number {
		const peak = clamp01;
		if (this.rig) {
			if (p === 'fft.level' || /^mix\d+\.out$/.test(p)) return this.bands.level;
			if (p === 'fft.bass') return this.bands.bass;
			return this.value[p] ?? NaN;
		}
		switch (p) {
			// the pitch as the choir line's cv: log-scaled into 0–1 across the spread's whole range
			case 'seq.cv':
				return this.gate ? 0.5 + Math.log2(this.pitch / BASE_HZ) / (2 * SPREAD_OCTAVES) : 0;
			case 'voice.out':
				return peak(this.gate * this.value['voice.gain'] * 4);
			case 'drone.out':
				return peak(this.value['drone.gain'] * 3);
			case 'incense.out':
				return peak(this.value['incense.gain'] * 5 * (0.4 + 0.6 * toPos(PARAMS['incense.cutoff'], this.cutoff())));
			case 'mix.out':
				return peak(this.bands.level);
			case 'sway.out':
				return this.sway;
			case 'fft.level':
				return this.bands.level;
			case 'fft.bass':
				return this.bands.bass;
			case 'incense.cutoff':
				return this.cutoff();
			default:
				return this.value[p] ?? NaN;
		}
	}

	/** the page's clock since the first tick (s) – the being's beat */
	get time(): number {
		return this.t0 < 0 ? 0 : this.last - this.t0;
	}

	/** One frame at `now` (s, performance.now() / 1000): the LFO, the melody, the mood, the stream. */
	tick(now: number): void {
		if (this.t0 < 0) this.t0 = this.last = now;
		const dt = Math.min(0.1, Math.max(0, now - this.last));
		this.last = now;
		this.swayPhase = (this.swayPhase + this.value['sway.rate'] * dt) % 1;
		this.sway = Math.sin(this.swayPhase * 2 * Math.PI) * this.value['sway.depth'];
		const core = this.core;
		// the fixed patch's melody and incense (a rig plays its own scene); nothing while its scene still loads
		if (core && this.engineReady && !this.rig) {
			// the melody: a random pitch on a continuous scale each step, the voice gliding there (no 12-tone grid);
			// spread 0 = the held drone at BASE_HZ
			const stepDur = 1 / this.value['melody.rate'];
			const t = core.context.currentTime;
			const step = Math.floor(t / stepDur);
			if (step !== this.lastStep) {
				this.lastStep = step;
				this.target = BASE_HZ * 2 ** ((Math.random() * 2 - 1) * this.value['melody.spread'] * SPREAD_OCTAVES);
				this.glideFrom = this.pitch;
				this.glideAt = t;
			}
			const g = Math.max(0.001, this.value['melody.glide'] * stepDur);
			this.pitch = this.glideFrom * (this.target / this.glideFrom) ** Math.min(1, (t - this.glideAt) / g);
			this.engineSet('voice.freq', [this.pitch]);
			if (!this.gate) {
				this.gate = 1;
				this.engineSet('env', [1]);
			}
			this.noises.get('incense')?.filter.frequency.setTargetAtTime(this.cutoff(), t, 0.03);
		}
		const m = this.moods.update(now, dt, this.bands.level, this.touch);
		if (m !== this.mood) {
			this.mood = m;
			this.onMood?.(m);
		}
		const changes = this.stream.sample(now, (p) => this.read(p), this.mood);
		if (changes) this.link?.send(changes);
	}

	/** Start the sound (needs a user gesture): live-core with the fixed patch or the rig, and the page's noises. */
	async start(files: EngineFiles): Promise<void> {
		const c = await LiveCore.start({ worker: files.worker, wasm: files.wasm, worklet: files.worklet, transport: files.transport ?? 'auto', lead: files.lead ?? defaultLead() });
		c.onAnalysis((a) => (this.bands = a));
		this.core = c;
		await this.loadEngine(c);
	}

	/** Stop the sound (live-core and the page's noises; the AudioContext closes). The deck keeps its values and rig and
	 *  can `start` again (#298). */
	async stop(): Promise<void> {
		const c = this.core;
		this.core = null;
		this.engineReady = false;
		this.loadGen++;
		for (const n of this.noises.values()) {
			try {
				n.src.stop();
			} catch {
				/* already stopped */
			}
		}
		this.noises.clear();
		this.bands = { level: 0, bass: 0, mid: 0, high: 0, spectrum: new Float32Array(0), reduction: 0 };
		await c?.stop();
	}

	/** (Re)load the sound: live-core's scene and the noises the page plays, then every slider's value. The engine is
	 *  quiet from here until the scene landed; a newer load (a second switch before this one landed) wins. */
	private async loadEngine(c: LiveCore): Promise<void> {
		const gen = ++this.loadGen;
		this.engineReady = false;
		for (const n of this.noises.values()) n.src.stop();
		this.noises.clear();
		const rig = this.rig;
		try {
			await c.load(rig ? rig.engine.scene : ENGINE_TOML);
		} catch (e) {
			if (gen === this.loadGen) this.o.onStatus?.(`sound: ${(e as Error).message ?? e}`);
			return;
		}
		if (gen !== this.loadGen) return;
		this.engineReady = true;
		this.gate = 0;
		if (rig) {
			for (const n of rig.engine.page) {
				// seq and other page kinds: not played yet (the voice holds its pitch)
				if (n.kind !== 'noise') continue;
				const num = (k: string, d: number) => (typeof n[k] === 'number' ? (n[k] as number) : d);
				this.noises.set(n.id, pinkNoise(c.context, num('cutoff', 4000), num('gain', 0), n.gain));
			}
		} else this.noises.set('incense', pinkNoise(c.context, this.cutoff(), this.value['incense.gain'], 1));
		for (const p of Object.keys(this.params)) this.setParam(p, this.value[p]);
	}

	close() {
		this.link?.close();
	}
}

/** Frames the sound keeps ahead of the speaker (#188): 3072 (64 ms) on a phone – a touch screen, where the worklet
 * ran dry under load –, else 2048. */
export function defaultLead(coarse = typeof matchMedia === 'function' && matchMedia('(pointer: coarse)').matches): number {
	return coarse ? 3072 : 2048;
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
