import type { AudioConfiguration, AudioEventsConfig } from '@scutoons/content';
import type { Action } from 'svelte/action';
import { getCurrentAudioContext } from '$lib/contexts/audio.svelte.ts';
import { handleMusicEvent } from './handlers/music.ts';
import { handleSampleEvent } from './handlers/samples.ts';

export const audioEvents: Action<HTMLElement, AudioEventsConfig | null | undefined> = (
	node,
	config
) => {
	const cleanups: Array<() => void> = [];

	function setup(cfg: AudioEventsConfig | null | undefined) {
		cleanups.forEach(fn => fn());
		cleanups.length = 0;

		if (!cfg || !cfg.events) return;

		const audioContext = getCurrentAudioContext();
		if (!audioContext) return;

		if (cfg.events.music?.tracks && cfg.events.music.tracks.length > 0) {
			const cleanup = handleMusicEvent(node, {
				index: cfg.index,
				slices: cfg.slices,
				audioContext
			});
			if (cleanup) cleanups.push(cleanup);
		}

		if (cfg.events.audio) {
			const cleanup = handleSampleEvent(node, cfg.events.audio, {
				audioContext
			});
			if (cleanup) cleanups.push(cleanup);
		}
	}

	setup(config);

	return {
		update: setup,
		destroy: () => cleanups.forEach(fn => fn())
	};
};

export const audio: Action<HTMLElement, AudioConfiguration | null | undefined> = (
	node,
	audioConfig
) => {
	const audioContext = getCurrentAudioContext();
	let currentConfig: AudioConfiguration | null | undefined = null;

	async function setup(config: AudioConfiguration | null | undefined) {
		if (!audioContext) return;
		if (config === currentConfig) return;

		currentConfig = config;

		if (config) {
			await audioContext.loadConfiguration(config);
			audioContext.startMusic();
		}
	}

	setup(audioConfig);

	return {
		update: setup,
		destroy: async () => {
			if (audioContext) {
				await audioContext.unload();
			}
		}
	};
};
