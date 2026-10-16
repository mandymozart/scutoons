import type { AdoptiveMusicPlayerConfiguration, AudioTrack } from '@scutoons/content';
import type { IAdaptiveMusicPlayer, IAudioManager } from './types.ts';
/**
 * The music's own audio output, one per page and never closed: Howler replaces its AudioContext now and then
 * (`Howler.unload()` on every chapter cleanup, and once on the first Howl when the sample rate isn't 44.1 kHz), which
 * would silence media elements bound to the old context. The master gain carries the app's mute / master volume.
 */
let output: { ctx: AudioContext; master: GainNode } | null | undefined;

/** the music's output (its context + master gain → speakers) – audio-effects.ts inserts a chain after the master */
export function musicOutput(): { ctx: AudioContext; master: GainNode } | null {
	if (output !== undefined) return output;
	const Ctx =
		typeof window === 'undefined'
			? undefined
			: (window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext);
	if (!Ctx) return (output = null);
	const ctx = new Ctx();
	const master = ctx.createGain();
	master.connect(ctx.destination);
	return (output = { ctx, master });
}

/**
 * One streamed music track: an `<audio>` element (downloads progressively, starts before the whole file is there)
 * routed through Web Audio (`MediaElementAudioSourceNode` → own gain → the music master gain), so fades are gain
 * ramps like before and volumes work on iOS (where `HTMLMediaElement.volume` is read-only, which rules out Howler's
 * `html5: true` for crossfades). Without Web Audio it falls back to the element volume.
 */
class StreamedTrack {
	readonly element: HTMLAudioElement;
	private gain: GainNode | null = null;
	private rampTimer: ReturnType<typeof setInterval> | null = null;
	private readonly src: string;
	/** the file is requested (a dormant track has no `src` yet: nothing is downloaded) */
	loading = false;

	/** `dormant`: created without its file – `load()` fetches it when a slice that raises it comes near (LOADING.md L2) */
	constructor(src: string, dormant = false) {
		const element = new Audio();
		// the CDN sends CORS headers; without crossOrigin the Web Audio graph would output silence
		element.crossOrigin = 'anonymous';
		element.loop = true;
		this.src = src;
		this.element = element;
		if (!dormant) this.load();

		const out = musicOutput();
		if (out) {
			const source = out.ctx.createMediaElementSource(element);
			this.gain = out.ctx.createGain();
			this.gain.gain.value = 0;
			source.connect(this.gain);
			this.gain.connect(out.master);
		} else {
			element.volume = 0;
		}
	}

	/** Starts the download (once). */
	load() {
		if (this.loading) return;
		this.loading = true;
		this.element.preload = 'auto';
		this.element.src = this.src;
	}

	/** Resolves when the element can start playing (or failed: logged, the chapter goes on without this track). */
	ready(id: string): Promise<void> {
		return new Promise((resolve) => {
			const done = () => {
				this.element.removeEventListener('canplay', done);
				this.element.removeEventListener('error', failed);
				resolve();
			};
			const failed = () => {
				console.error(`Failed to load track: ${id}`, this.element.error);
				done();
			};
			if (this.element.readyState >= HTMLMediaElement.HAVE_FUTURE_DATA) return resolve();
			this.element.addEventListener('canplay', done);
			this.element.addEventListener('error', failed);
		});
	}

	volume(): number {
		return this.gain ? this.gain.gain.value : this.element.volume;
	}

	/** Linear ramp from the current volume to `to` over `ms`. */
	fade(to: number, ms: number) {
		if (this.gain) {
			const param = this.gain.gain;
			const now = this.gain.context.currentTime;
			param.cancelScheduledValues(now);
			param.setValueAtTime(param.value, now);
			param.linearRampToValueAtTime(to, now + ms / 1000);
			return;
		}
		if (this.rampTimer) clearInterval(this.rampTimer);
		const from = this.element.volume;
		const start = performance.now();
		this.rampTimer = setInterval(() => {
			const k = Math.min(1, (performance.now() - start) / ms);
			this.element.volume = from + (to - from) * k;
			if (k >= 1 && this.rampTimer) {
				clearInterval(this.rampTimer);
				this.rampTimer = null;
			}
		}, 50);
	}

	play() {
		// a dormant track waits for its file (`want` starts it once it is there)
		if (!this.loading) return;
		this.element.play().catch((error) => console.warn('Music playback blocked until a user gesture', error));
	}

	/** Joins the reference track at its position (in seconds) – parallel stems of one piece stay in step. */
	syncTo(position: number) {
		const duration = this.element.duration;
		if (!(duration > 0) || !(position > 0)) return;
		this.element.currentTime = position % duration;
	}

	/** where it plays, in seconds */
	position(): number {
		return this.element.currentTime;
	}

	/** without Web Audio: the app's mute */
	setMuted(muted: boolean) {
		this.element.muted = muted;
	}

	/** DEV checks */
	state() {
		return { paused: this.element.paused, time: this.element.currentTime, ready: this.element.readyState };
	}

	pause() {
		this.element.pause();
	}

	stop() {
		this.element.pause();
		this.element.currentTime = 0;
	}

	/** Stops the download too. */
	unload() {
		if (this.rampTimer) clearInterval(this.rampTimer);
		this.element.pause();
		this.element.removeAttribute('src');
		this.element.load();
		this.gain?.disconnect();
	}
}

/**
 * A short loop (a file named `…-loop.<rate>.mp3`, ~30 s – agents/ASSETS.md "Music loops", owner 2026-10-16): fetched
 * whole, decoded once and looped sample-exactly by Web Audio (`AudioBufferSourceNode.loop`). An `<audio loop>` leaves a
 * tiny gap at every wrap on some browsers (Safari ignores the MP3's gapless info) – every 30 s that is audible. Same
 * interface as `StreamedTrack`; only with a music output (Web Audio), else the streamed fallback is used.
 */
class BufferedTrack {
	private readonly gain: GainNode;
	private readonly ctx: AudioContext;
	private buffer: AudioBuffer | null = null;
	private source: AudioBufferSourceNode | null = null;
	private decoded: Promise<void> | null = null;
	private startedAt = 0;
	private offset = 0;
	private playing = false;
	loading = false;

	constructor(
		private readonly src: string,
		out: { ctx: AudioContext; master: GainNode },
		dormant = false
	) {
		this.ctx = out.ctx;
		this.gain = out.ctx.createGain();
		this.gain.gain.value = 0;
		this.gain.connect(out.master);
		if (!dormant) this.load();
	}

	load() {
		if (this.loading) return;
		this.loading = true;
		this.decoded = fetch(this.src, { mode: 'cors' })
			.then((res) => (res.ok ? res.arrayBuffer() : Promise.reject(new Error(`${res.status}`))))
			.then((bytes) => this.ctx.decodeAudioData(bytes))
			.then((buffer) => {
				this.buffer = buffer;
				if (this.playing && !this.source) this.startSource();
			});
	}

	ready(id: string): Promise<void> {
		return (this.decoded ?? Promise.resolve()).catch((error) => console.error(`Failed to load track: ${id}`, error));
	}

	private startSource() {
		if (!this.buffer) return;
		const source = this.ctx.createBufferSource();
		source.buffer = this.buffer;
		source.loop = true;
		source.connect(this.gain);
		const at = this.offset % this.buffer.duration;
		source.start(0, at);
		this.startedAt = this.ctx.currentTime - at;
		this.source = source;
	}

	private stopSource() {
		if (!this.source) return;
		this.offset = this.position();
		this.source.stop();
		this.source.disconnect();
		this.source = null;
	}

	position(): number {
		if (!this.buffer) return this.offset;
		return this.source ? (this.ctx.currentTime - this.startedAt) % this.buffer.duration : this.offset;
	}

	volume(): number {
		return this.gain.gain.value;
	}

	fade(to: number, ms: number) {
		const param = this.gain.gain;
		const now = this.ctx.currentTime;
		param.cancelScheduledValues(now);
		param.setValueAtTime(param.value, now);
		param.linearRampToValueAtTime(to, now + ms / 1000);
	}

	play() {
		if (!this.loading) return;
		this.playing = true;
		if (!this.source) this.startSource();
	}

	pause() {
		this.stopSource();
		this.playing = false;
	}

	stop() {
		this.pause();
		this.offset = 0;
	}

	syncTo(position: number) {
		if (!(position > 0)) return;
		const wasPlaying = !!this.source;
		this.stopSource();
		this.offset = this.buffer ? position % this.buffer.duration : position;
		if (wasPlaying) this.startSource();
	}

	setMuted() {
		/* the master gain mutes it */
	}

	unload() {
		this.pause();
		this.buffer = null;
		this.gain.disconnect();
	}

	state() {
		return { paused: !this.source, time: this.position(), ready: this.buffer ? 4 : 0 };
	}
}

type MusicTrack = StreamedTrack | BufferedTrack;

/** a short loop file (`…-loop.<rate>.mp3`): decoded and looped sample-exactly; anything else streams */
export const isLoopFile = (path: string) => /-loop(\.\d+)?\.(mp3|m4a|ogg|opus|wav)(\?.*)?$/i.test(path);

function createTrack(src: string, dormant: boolean): MusicTrack {
	const out = musicOutput();
	return out && isLoopFile(src) ? new BufferedTrack(src, out, dormant) : new StreamedTrack(src, dormant);
}

/**
 * AdaptiveMusicPlayer - parallel-track emotion-based music system
 * Manages smooth crossfading between emotional states based on slice data.
 * The tracks are streamed (agents/PERFORMANCE.md P1): they start as soon as enough is buffered.
 */
export class AdaptiveMusicPlayer implements IAdaptiveMusicPlayer {
	audioManager: IAudioManager;
	tracks: Map<string, MusicTrack>;
	currentVolumes: Map<string, number>;
	isPlaying: boolean;
	defaultTransitionDuration: number;
	defaultTrackId: string | null;
	constructor(audioManager: IAudioManager) {
		this.audioManager = audioManager;
		this.tracks = new Map(); // Map<trackId, MusicTrack>
		this.currentVolumes = new Map(); // Map<trackId, number>
		this.isPlaying = false;
		this.defaultTransitionDuration = 3000; // ms
		// DEV only: the headless checks read the tracks' state
		if (import.meta.env.DEV && typeof window !== 'undefined') (window as unknown as Record<string, unknown>).__scutoonsMusic = () => ({ ctx: musicOutput()?.ctx.state, playing: this.isPlaying, tracks: [...this.tracks].map(([id, t]) => { const s = t.state(); return { id, paused: s.paused, time: Math.round(s.time * 10) / 10, volume: Math.round(t.volume() * 100) / 100, ready: s.ready, loading: t.loading }; }) });
		// DEV only: the checks ask for a dormant track and set volumes like a slice would
		if (import.meta.env.DEV && typeof window !== 'undefined') (window as unknown as Record<string, unknown>).__scutoonsMusicDo = { want: (ids: string[]) => this.want(ids), volumes: (v: { id: string; volume: number }[]) => this.updateVolumes(v, 300) };
		this.defaultTrackId = null; // Default track to play at full volume
	}

	/**
	 * `eager`: the tracks to fetch now (the default one + those the first slices raise); null = all (the old way, the
	 * `generous` data tier). The others stay dormant until `want` (LOADING.md L2: ch1's second track is 6.6 MB and plays
	 * minutes later). Resolves when the eager ones can play.
	 */
	async loadTracks(musicConfig: AdoptiveMusicPlayerConfiguration, eager: string[] | null = null) {
		if (!musicConfig || !musicConfig.tracks || musicConfig.tracks.length === 0) return;

		this.defaultTransitionDuration = musicConfig.transitionDuration || 3000;

		if (musicConfig.defaultTrack) {
			this.defaultTrackId = musicConfig.defaultTrack;
		} else if (musicConfig.tracks.length > 0) {
			this.defaultTrackId = musicConfig.tracks[0].id;
		}

		const now = (id: string) => eager === null || id === this.defaultTrackId || eager.includes(id);
		await Promise.all(musicConfig.tracks.map((trackConfig) => this.loadTrack(trackConfig, !now(trackConfig.id))));
		console.log(`Loaded ${[...this.tracks.values()].filter((t) => t.loading).length} of ${this.tracks.size} music tracks (default: ${this.defaultTrackId})`);
	}

	loadTrack(trackConfig: AudioTrack, dormant = false): Promise<void> {
		const track = createTrack(trackConfig.path, dormant);
		this.tracks.set(trackConfig.id, track);
		this.currentVolumes.set(trackConfig.id, 0);
		return dormant ? Promise.resolve() : track.ready(trackConfig.id);
	}

	/**
	 * A slice that raises these tracks is near: fetch the dormant ones; once one can play it joins the default track in
	 * step (when the music plays) and fades to the volume asked for meanwhile.
	 */
	want(ids: string[]) {
		for (const id of ids) {
			const track = this.tracks.get(id);
			if (!track || track.loading) continue;
			track.load();
			void track.ready(id).then(() => {
				if (this.tracks.get(id) !== track || !this.isPlaying) return;
				const reference = this.defaultTrackId ? this.tracks.get(this.defaultTrackId) : undefined;
				if (reference && reference !== track && reference.loading) track.syncTo(reference.position());
				track.play();
				const volume = this.currentVolumes.get(id) ?? 0;
				if (volume > 0) track.fade(volume, this.defaultTransitionDuration);
			});
		}
	}

	start() {
		if (this.isPlaying) return;
		this.tracks.forEach((track) => track.play());
		this.isPlaying = true;
	}

	updateVolumes(trackVolumes: { id: string; volume: number }[], duration?: number) {
		if (!this.isPlaying) return;

		const transitionDuration = duration || this.defaultTransitionDuration;

		trackVolumes.forEach((trackVolume) => {
			const track = this.tracks.get(trackVolume.id);
			if (!track) {
				return;
			}

			const targetVolume = Math.max(0, Math.min(1, trackVolume.volume));
			this.currentVolumes.set(trackVolume.id, targetVolume);
			// asked for before its file came (a fast scroll): fetch it now, it fades in when it can play (`want`)
			if (!track.loading) {
				if (targetVolume > 0) this.want([trackVolume.id]);
				return;
			}
			track.fade(targetVolume, transitionDuration);
		});
	}

	pause() {
		if (!this.isPlaying) return;
		this.tracks.forEach((track) => track.pause());
		this.isPlaying = false;
	}

	resume() {
		if (this.isPlaying) return;
		this.tracks.forEach((track) => track.play());
		this.isPlaying = true;
	}

	stop() {
		this.tracks.forEach((track) => track.stop());
		this.isPlaying = false;
	}

	/** Stops and releases the tracks (no further download). */
	unload() {
		this.tracks.forEach((track) => track.unload());
		this.tracks.clear();
		this.isPlaying = false;
	}

	/** Whether the music output may play (browsers keep an AudioContext suspended until a user gesture). */
	isUnlocked(): boolean {
		const out = musicOutput();
		return !out || out.ctx.state === 'running';
	}

	/** Resume the music output (call it inside a user gesture). */
	unlock() {
		const out = musicOutput();
		if (out && out.ctx.state !== 'running') out.ctx.resume().catch(() => {});
	}

	/** The app's master volume / mute for the music (Howler's apply to the samples). */
	setMasterVolume(volume: number, muted: boolean) {
		const out = musicOutput();
		if (!out) {
			this.tracks.forEach((track) => track.setMuted(muted));
			return;
		}
		out.master.gain.setValueAtTime(muted ? 0 : volume, out.ctx.currentTime);
	}

	fadeOutAndStop(duration?: number): Promise<void> {
		return new Promise((resolve) => {
			if (!this.isPlaying) {
				this.unload();
				resolve();
				return;
			}
			const fadeDuration = duration !== undefined && duration !== null ? duration : this.defaultTransitionDuration;
			this.tracks.forEach((track, trackId) => {
				if (track.volume() > 0) {
					track.fade(0, fadeDuration);
					this.currentVolumes.set(trackId, 0);
				}
			});

			setTimeout(() => {
				this.unload();
				resolve();
			}, fadeDuration);
		});
	}

	getCurrentVolumes(): Map<string, number> {
		return new Map(this.currentVolumes);
	}

	isCurrentlyPlaying(): boolean {
		return this.isPlaying;
	}

	getDefaultTrackId(): string | null {
		return this.defaultTrackId;
	}

	getTrackIds(): string[] {
		return Array.from(this.tracks.keys());
	}
}
