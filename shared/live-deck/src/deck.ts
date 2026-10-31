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
import { BASE_HZ, deckGraph, ENGINE_TOML, ladderFor, MELODY, Moods, PARAMS, SPREAD_OCTAVES, STREAMED, toPos } from './scene.ts';

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

/** who turned a value: the page's controls or the being's 3D buttons (the page moves its sliders for the latter) */
export type Hand = 'page' | 'being';

const clamp01 = (x: number) => Math.min(1, Math.max(0, x));

export class Deck {
	/** every parameter's value (PARAMS + MELODY) */
	readonly value: Record<string, number> = Object.fromEntries(Object.entries({ ...PARAMS, ...MELODY }).map(([k, s]) => [k, s.value]));
	character: string;
	portraits: Portraits = {};
	readonly moods = new Moods(['neutral'], 1.5, BPM);
	mood = 'neutral';
	bands: Analysis = { level: 0, bass: 0, mid: 0, high: 0, spectrum: new Float32Array(0) };
	/** a finger on a pad (0 / 1): lifts the energy a little */
	touch = 0;
	readonly link: DeckLink | null;
	/** #195: the answers to the conductor's pushes (null without a CDN) */
	readonly sync: BrowserSync | null;
	readonly stream = new ValueStream(STREAMED);
	core: LiveCore | null = null;
	/** every value change, with whose hand (the page's controls follow the being's) */
	onValue: ((path: string, value: number, hand: Hand) => void) | null = null;
	onMood: ((mood: string) => void) | null = null;
	private noise: { filter: BiquadFilterNode; gain: GainNode } | null = null;
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
		return deckGraph(this.character, [...this.moods.moods, ...have.filter((m) => !this.moods.moods.includes(m))]);
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
		const core = this.core;
		if (!core) return;
		if (path === 'voice.ratio' || path === 'voice.index') core.set(path, [v]);
		else if (path === 'voice.gain') core.set('vgain.b', [v]);
		else if (path === 'drone.freq' || path === 'drone.gain') {
			core.set('drone', [this.value['drone.freq'], this.value['drone.gain']]);
			core.set('fifth', [this.value['drone.freq'] * 1.5, this.value['drone.gain'] * 0.3]);
		} else if (path === 'incense.gain' && this.noise) this.noise.gain.gain.setTargetAtTime(v, this.noise.gain.context.currentTime, 0.05);
	}

	/** the incense's cutoff now: its slider swayed by the LFO */
	cutoff(): number {
		return Math.min(12000, this.value['incense.cutoff'] * 2 ** (this.sway * 1.5));
	}

	/** a streamed path's value now (the hoses' sources are the page's own: its melody, its levels, its LFO) */
	read(p: string): number {
		const peak = clamp01;
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
		if (core) {
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
			core.set('voice.freq', [this.pitch]);
			if (!this.gate) {
				this.gate = 1;
				core.set('env', [1]);
			}
			this.noise?.filter.frequency.setTargetAtTime(this.cutoff(), t, 0.03);
		}
		const m = this.moods.update(now, dt, this.bands.level, this.touch);
		if (m !== this.mood) {
			this.mood = m;
			this.onMood?.(m);
		}
		const changes = this.stream.sample(now, (p) => this.read(p), this.mood);
		if (changes) this.link?.send(changes);
	}

	/** Start the sound (needs a user gesture): live-core with the trimmed cathedral, and the incense noise. */
	async start(files: EngineFiles): Promise<void> {
		const c = await LiveCore.start({ worker: files.worker, wasm: files.wasm, worklet: files.worklet, transport: files.transport ?? 'auto', lead: files.lead ?? 2048 });
		await c.load(ENGINE_TOML);
		c.onAnalysis((a) => (this.bands = a));
		const ctx = c.context;
		const buf = ctx.createBuffer(1, ctx.sampleRate * 2, ctx.sampleRate);
		const d = buf.getChannelData(0);
		let b0 = 0,
			b1 = 0,
			b2 = 0;
		for (let i = 0; i < d.length; i++) {
			// pink-ish: three leaky integrators of white noise (cathedral.toml's incense is pink)
			const w = Math.random() * 2 - 1;
			b0 = 0.99765 * b0 + w * 0.099;
			b1 = 0.963 * b1 + w * 0.2965;
			b2 = 0.57 * b2 + w * 1.0527;
			d[i] = (b0 + b1 + b2 + w * 0.1848) * 0.2;
		}
		const src = Object.assign(ctx.createBufferSource(), { buffer: buf, loop: true });
		const filter = Object.assign(ctx.createBiquadFilter(), { type: 'lowpass' as BiquadFilterType });
		const gain = ctx.createGain();
		gain.gain.value = this.value['incense.gain'];
		src.connect(filter).connect(gain).connect(ctx.destination);
		src.start();
		this.noise = { filter, gain };
		this.core = c;
		for (const p of Object.keys(PARAMS)) this.setParam(p, this.value[p]);
	}

	close() {
		this.link?.close();
	}
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
