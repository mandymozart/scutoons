import type { AudioConfiguration } from '@scutoons/content';
import { SLICE_PLAYER_EVENT, type SliceAudioPlayer, type SlicePlayerDetail } from '@scutoons/reader';
import type { Action } from 'svelte/action';
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
		// the announced tracks while the music still loads, so the volumes of the slices in view are kept
		getTrackIds: () => audioContext.getTrackIds(),
		updateMusicVolumes: (tracks) => audioContext.updateMusicVolumes(tracks)
	};
}

/** Wait this long at most for the first slices before the audio starts loading anyway. */
const FIRST_SLICES_MAX_WAIT_MS = 4000;

/**
 * Resolves when the media of the first `count` slices inside `node` has loaded (or failed), or after `maxWait`:
 * the chapter's audio (megabytes) must not compete with the first images (agents/PERFORMANCE.md P1).
 */
export function afterFirstSlices(node: HTMLElement, count = 2, maxWait = FIRST_SLICES_MAX_WAIT_MS): Promise<void> {
	return new Promise((resolve) => {
		const timer = setTimeout(resolve, maxWait);
		const done = () => {
			clearTimeout(timer);
			resolve();
		};
		requestAnimationFrame(() => {
			const media = [...node.querySelectorAll<HTMLElement>('.slice')]
				.slice(0, count)
				.map((slice) => slice.querySelector<HTMLImageElement | HTMLVideoElement>('video, img'))
				.filter((el): el is HTMLImageElement | HTMLVideoElement => !!el);
			Promise.all(
				media.map((el) => {
					const ready = el instanceof HTMLImageElement ? el.complete : el.readyState >= 2;
					if (ready) return;
					return new Promise<void>((ok) => {
						el.addEventListener(el instanceof HTMLImageElement ? 'load' : 'loadeddata', () => ok(), { once: true });
						el.addEventListener('error', () => ok(), { once: true });
						// a failing <source> reports on itself, not on the <video>
						el.querySelector('source:last-of-type')?.addEventListener('error', () => ok(), { once: true });
					});
				})
			).then(done);
		});
	});
}

export const audio: Action<HTMLElement, AudioConfiguration | null | undefined> = (
	node,
	audioConfig
) => {
	const audioContext = getCurrentAudioContext();
	let currentConfig: AudioConfiguration | null | undefined = null;
	let destroyed = false;

	async function setup(config: AudioConfiguration | null | undefined) {
		if (!audioContext) return;
		if (config === currentConfig) return;

		currentConfig = config;
		audioContext.expect(config ?? null);

		if (config) {
			await afterFirstSlices(node);
			if (destroyed || config !== currentConfig) return;
			await audioContext.loadConfiguration(config);
			// left while loading: the destroy's unload ran before anything was loaded
			if (destroyed) return void audioContext.unload();
			if (config !== currentConfig) return;
			audioContext.startMusic();
		}
	}

	setup(audioConfig);

	// a player video slice (a music video) has its own sound: the chapter's music pauses while it plays
	const onPlayer = (event: Event) => {
		const detail = (event as CustomEvent<SlicePlayerDetail>).detail;
		if (detail && detail.state !== 'time') audioContext?.holdForPlayer(detail.playing);
	};
	node.addEventListener(SLICE_PLAYER_EVENT, onPlayer);

	return {
		update: setup,
		destroy: async () => {
			destroyed = true;
			node.removeEventListener(SLICE_PLAYER_EVENT, onPlayer);
			if (audioContext) {
				await audioContext.unload();
			}
		}
	};
};
