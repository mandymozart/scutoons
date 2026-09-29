import type { AudioConfiguration } from '@scutoons/content';
import type { SliceAudioPlayer } from '@scutoons/reader';
import type { Action } from 'svelte/action';
import type { AdaptiveMusicPlayer } from '$lib/audio/AdaptiveMusicPlayer.ts';
import { getCurrentAudioContext } from '$lib/contexts/audio.svelte.ts';

/**
 * The current audio context as the player `@scutoons/reader`'s `sliceAudio` action expects (null = no audio).
 * Resolved when the slice sets up its action, like the former `audioEvents` action did.
 */
export function audioPlayer(): SliceAudioPlayer | null {
	const audioContext = getCurrentAudioContext();
	if (!audioContext) return null;
	return {
		triggerSample: (sample) => audioContext.triggerSample(sample),
		getTrackIds: () =>
			(audioContext.manager.getMusicPlayer() as AdaptiveMusicPlayer | null)?.getTrackIds() ?? null,
		updateMusicVolumes: (tracks) => audioContext.updateMusicVolumes(tracks)
	};
}

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
