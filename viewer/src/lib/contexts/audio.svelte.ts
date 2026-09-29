import { AudioManager } from '$lib/audio/AudioManager.ts';
import type { AudioConfiguration, Sample } from '@scutoons/content';
import { getContext, setContext } from 'svelte';
import { asset } from '$lib/utils/cdn.ts';
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
	loadConfiguration: (audioConfig: AudioConfiguration) => Promise<void>;
	unload: () => Promise<void>;
	startMusic: () => Promise<void>;
	pauseMusic: () => void;
	resumeMusic: () => void;
	updateMusicVolumes: (volumes: { id: string; volume: number }[], duration?: number) => void;
	triggerSample: (sample: Sample) => void;
	toggleMute: () => void;
	setVolume: (volume: number) => void;
};

function transformAudioPaths(config: AudioConfiguration, toonId: string): AudioConfiguration {
	const transformed = { ...config };

	if (transformed.music?.tracks) {
		transformed.music = {
			...transformed.music,
			tracks: transformed.music.tracks.map((track) => ({
				...track,
				path: asset(`${toonId}/${track.path}`)
			}))
		};
	}

	if (transformed.sampleLibrary) {
		transformed.sampleLibrary = transformed.sampleLibrary.map((sample) => ({
			...sample,
			path: asset(`${toonId}/${sample.path}`)
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

	const context = {
		manager,
		state,

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
			if (!state.isInitialized) return;

			await manager.cleanup(1000);
			state.isInitialized = false;
			state.currentConfig = null;
		},

		async startMusic() {
			const player = manager.getMusicPlayer();
			if (player && !player.isCurrentlyPlaying()) {
				if (!manager.isAudioUnlocked()) {
					const resumeAudio = async () => {
						if (Howler.ctx && Howler.ctx.state === 'suspended') {
							await Howler.ctx.resume();
							if (player && !player.isCurrentlyPlaying()) {
								player.start();
								state.isPlaying = true;
							}
						}
						document.removeEventListener('click', resumeAudio);
						document.removeEventListener('keydown', resumeAudio);
						document.removeEventListener('touchstart', resumeAudio);
					};
					document.addEventListener('click', resumeAudio, { once: true });
					document.addEventListener('keydown', resumeAudio, { once: true });
					document.addEventListener('touchstart', resumeAudio, { once: true });
				} else {
					player.start();
					state.isPlaying = true;
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

		updateMusicVolumes(volumes: { id: string; volume: number }[], duration?: number) {
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
