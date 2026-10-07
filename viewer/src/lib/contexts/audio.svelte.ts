import { AudioManager } from '$lib/audio/AudioManager.ts';
import type { AudioConfiguration, Sample } from '@scutoons/content';
import { getContext, setContext } from 'svelte';
import { toonUrl } from '$lib/utils/cdn.ts';
import { mediaQuality } from '$lib/media-quality.svelte.ts';
import { pickQualityVariant, type AudioTrack } from '@scutoons/content';
import { Howler } from 'howler';

const AUDIO_KEY = Symbol('audio');

export type AudioContext = {
	manager: AudioManager;
	state: {
		isInitialized: boolean;
		isMuted: boolean;
		masterVolume: number;
		isPlaying: boolean;
		currentConfig: AudioConfiguration | null;
	};
	/**
	 * Announce the chapter's music before it loads (the loading waits for the first slices): the slices' music
	 * volumes are remembered from now on and applied when the music starts.
	 */
	expect: (audioConfig: AudioConfiguration | null) => void;
	/** track ids of the loaded music, or of the announced one while it loads; null = no music */
	getTrackIds: () => string[] | null;
	loadConfiguration: (audioConfig: AudioConfiguration) => Promise<void>;
	unload: () => Promise<void>;
	startMusic: () => Promise<void>;
	pauseMusic: () => void;
	resumeMusic: () => void;
	/**
	 * A player video (`SliceVideo.player`) plays (true) or stopped (false): the music pauses while it plays and resumes
	 * afterwards (also music that would have started meanwhile).
	 */
	holdForPlayer: (playing: boolean) => void;
	updateMusicVolumes: (volumes: { id: string; volume: number }[], duration?: number) => void;
	triggerSample: (sample: Sample) => void;
	toggleMute: () => void;
	setVolume: (volume: number) => void;
};

/**
 * The file a track / sample plays: the reader's media quality level (96k / 128k / 320k, falling back to the next lower
 * one); `path` (the 128k default) while the level isn't known yet or without variants. Chosen when the chapter's audio
 * loads – a changed setting applies from the next chapter.
 */
function audioFile(track: AudioTrack): string {
	const level = mediaQuality.level;
	return level ? pickQualityVariant({ src: track.path, variants: track.variants }, level) : track.path;
}

function transformAudioPaths(config: AudioConfiguration, toonId: string): AudioConfiguration {
	const transformed = { ...config };

	if (transformed.music?.tracks) {
		transformed.music = {
			...transformed.music,
			tracks: transformed.music.tracks.map((track) => ({
				...track,
				path: toonUrl(toonId, audioFile(track))
			}))
		};
	}

	if (transformed.sampleLibrary) {
		transformed.sampleLibrary = transformed.sampleLibrary.map((sample) => ({
			...sample,
			path: toonUrl(toonId, audioFile(sample))
		}));
	}

	return transformed;
}

let _currentContext: AudioContext | undefined;

export function setAudioContext(toonId: string): AudioContext {
	const manager = new AudioManager();

	const state = $state({
		isInitialized: false,
		isMuted: false,
		masterVolume: 1.0,
		isPlaying: false,
		currentConfig: null as AudioConfiguration | null
	});

	/** track ids of the announced music (see `expect`) */
	let expectedTrackIds: string[] | null = null;
	/** the last volume the slices asked for per track, applied when the music starts */
	const requestedVolumes = new Map<string, number>();
	/** a player video plays: the music waits */
	let held = false;
	/** the music was playing (or wanted to start) when the hold began */
	let resumeAfterHold = false;

	function startPlayer(player: NonNullable<ReturnType<AudioManager['getMusicPlayer']>>) {
		if (player.isCurrentlyPlaying()) return;
		if (held) {
			resumeAfterHold = true;
			return;
		}
		player.start();
		state.isPlaying = true;
		// volumes the slices in view asked for while the music was loading / waiting for a gesture
		if (requestedVolumes.size > 0) {
			player.updateVolumes([...requestedVolumes].map(([id, volume]) => ({ id, volume })));
		}
	}

	const context = {
		manager,
		state,

		expect(audioConfig: AudioConfiguration | null) {
			expectedTrackIds = audioConfig?.music?.tracks?.map((track) => track.id) ?? null;
			requestedVolumes.clear();
		},

		getTrackIds() {
			return manager.getMusicPlayer()?.getTrackIds() ?? expectedTrackIds;
		},

		async loadConfiguration(audioConfig: AudioConfiguration) {

			if (state.isInitialized) {
				await manager.cleanup(1000);
				state.isInitialized = false;
			}

			const transformedConfig = transformAudioPaths(audioConfig, toonId);

			await manager.init(transformedConfig);
			state.isInitialized = true;
			state.currentConfig = audioConfig;
		},

		async unload() {
			expectedTrackIds = null;
			held = false;
			resumeAfterHold = false;
			requestedVolumes.clear();
			if (!state.isInitialized) return;

			await manager.cleanup(1000);
			state.isInitialized = false;
			state.currentConfig = null;
		},

		async startMusic() {
			const player = manager.getMusicPlayer();
			if (player && !player.isCurrentlyPlaying()) {
				if (!manager.isAudioUnlocked()) {
					// the first gesture starts the (streamed) tracks synchronously, while the gesture still counts
					// for autoplay, and resumes the audio context (Howler's own unlock may already be doing that)
					const resumeAudio = () => {
						document.removeEventListener('click', resumeAudio);
						document.removeEventListener('keydown', resumeAudio);
						document.removeEventListener('touchstart', resumeAudio);
						if (manager.getMusicPlayer() !== player) return; // the chapter was left meanwhile
						player.unlock();
						startPlayer(player);
						if (Howler.ctx && Howler.ctx.state !== 'running') Howler.ctx.resume();
					};
					document.addEventListener('click', resumeAudio, { once: true });
					document.addEventListener('keydown', resumeAudio, { once: true });
					document.addEventListener('touchstart', resumeAudio, { once: true });
				} else {
					startPlayer(player);
				}
			}
		},

		pauseMusic() {
			const player = manager.getMusicPlayer();
			if (player) {
				player.pause();
				state.isPlaying = false;
			}
		},

		resumeMusic() {
			const player = manager.getMusicPlayer();
			if (player) {
				player.resume();
				state.isPlaying = true;
			}
		},

		holdForPlayer(playing: boolean) {
			if (playing === held) return;
			held = playing;
			const player = manager.getMusicPlayer();
			if (playing) {
				if (player?.isCurrentlyPlaying()) {
					player.pause();
					state.isPlaying = false;
					resumeAfterHold = true;
				}
				return;
			}
			const resume = resumeAfterHold;
			resumeAfterHold = false;
			if (resume && player && !player.isCurrentlyPlaying()) startPlayer(player);
		},

		updateMusicVolumes(volumes: { id: string; volume: number }[], duration?: number) {
			for (const { id, volume } of volumes) requestedVolumes.set(id, volume);
			const player = manager.getMusicPlayer();
			if (player) {
				player.updateVolumes(volumes, duration);
			}
		},

		triggerSample(sample: Sample) {
			const sampleManager = manager.getSampleManager();
			if (sampleManager) {
				sampleManager.trigger(sample);
			}
		},

		toggleMute() {
			manager.toggleMute();
			state.isMuted = manager.isMutedState();
		},

		setVolume(volume: number) {
			manager.setMasterVolume(volume);
			state.masterVolume = volume;
		}
	};

	setContext(AUDIO_KEY, context);
	_currentContext = context;
	return context;
}

export function getAudioContext() {
	return getContext<AudioContext>(AUDIO_KEY);
}

export function getCurrentAudioContext() {
	return _currentContext;
}
