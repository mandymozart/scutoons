import type { AdoptiveMusicPlayerConfiguration, AudioTrack } from '@scutoons/content';
import type { IAdaptiveMusicPlayer, IAudioManager } from './types.ts';
/**
 * The music's own audio output, one per page and never closed: Howler replaces its AudioContext now and then
 * (`Howler.unload()` on every chapter cleanup, and once on the first Howl when the sample rate isn't 44.1 kHz), which
 * would silence media elements bound to the old context. The master gain carries the app's mute / master volume.
 */
let output: { ctx: AudioContext; master: GainNode } | null | undefined;

function musicOutput(): { ctx: AudioContext; master: GainNode } | null {
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

	constructor(src: string) {
		const element = new Audio();
		// the CDN sends CORS headers; without crossOrigin the Web Audio graph would output silence
		element.crossOrigin = 'anonymous';
		element.preload = 'auto';
		element.loop = true;
		element.src = src;
		this.element = element;

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
		this.element.play().catch((error) => console.warn('Music playback blocked until a user gesture', error));
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
 * AdaptiveMusicPlayer - parallel-track emotion-based music system
 * Manages smooth crossfading between emotional states based on slice data.
 * The tracks are streamed (agents/PERFORMANCE.md P1): they start as soon as enough is buffered.
 */
export class AdaptiveMusicPlayer implements IAdaptiveMusicPlayer {
	audioManager: IAudioManager;
	tracks: Map<string, StreamedTrack>;
	currentVolumes: Map<string, number>;
	isPlaying: boolean;
	defaultTransitionDuration: number;
	defaultTrackId: string | null;
	constructor(audioManager: IAudioManager) {
		this.audioManager = audioManager;
		this.tracks = new Map(); // Map<trackId, StreamedTrack>
		this.currentVolumes = new Map(); // Map<trackId, number>
		this.isPlaying = false;
		this.defaultTransitionDuration = 3000; // ms
		this.defaultTrackId = null; // Default track to play at full volume
	}

	async loadTracks(musicConfig: AdoptiveMusicPlayerConfiguration) {
		if (!musicConfig || !musicConfig.tracks || musicConfig.tracks.length === 0) return;

		this.defaultTransitionDuration = musicConfig.transitionDuration || 3000;

		if (musicConfig.defaultTrack) {
			this.defaultTrackId = musicConfig.defaultTrack;
		} else if (musicConfig.tracks.length > 0) {
			this.defaultTrackId = musicConfig.tracks[0].id;
		}

		await Promise.all(musicConfig.tracks.map((trackConfig) => this.loadTrack(trackConfig)));
		console.log(`Loaded ${this.tracks.size} music tracks (default: ${this.defaultTrackId})`);
	}

	loadTrack(trackConfig: AudioTrack): Promise<void> {
		const track = new StreamedTrack(trackConfig.path);
		this.tracks.set(trackConfig.id, track);
		this.currentVolumes.set(trackConfig.id, 0);
		return track.ready(trackConfig.id);
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
			track.fade(targetVolume, transitionDuration);
			this.currentVolumes.set(trackVolume.id, targetVolume);
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
			this.tracks.forEach((track) => (track.element.muted = muted));
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
