import type { Action } from 'svelte/action';
import type { AudioEventsConfig, MusicEventTrack, Sample } from '@scutoons/content';
import { handleMusicEvent } from './music.ts';
import { handleSampleEvent } from './samples.ts';

/** What a slice needs from the app's audio player (the viewer adapts its audio context to this). */
export interface SliceAudioPlayer {
	triggerSample(sample: Sample): void;
	/** track ids of the loaded adaptive music player, null while there is none */
	getTrackIds(): string[] | null;
	updateMusicVolumes(tracks: MusicEventTrack[]): void;
}

export interface SliceAudioConfig extends AudioEventsConfig {
	/** null = no audio (nothing is wired) */
	player: SliceAudioPlayer | null | undefined;
}

/**
 * Slice audio events: music volumes of this slice while it is in view (last known volume per track, searched
 * backwards through the slices) and its sample trigger (click / hover / intersect, optionally inside a rect).
 */
export const sliceAudio: Action<HTMLElement, SliceAudioConfig | null | undefined> = (node, config) => {
	const cleanups: Array<() => void> = [];

	function setup(cfg: SliceAudioConfig | null | undefined) {
		cleanups.forEach((fn) => fn());
		cleanups.length = 0;

		if (!cfg || !cfg.events) return;

		const player = cfg.player;
		if (!player) return;

		if (cfg.events.music?.tracks && cfg.events.music.tracks.length > 0) {
			const cleanup = handleMusicEvent(node, {
				index: cfg.index,
				slices: cfg.slices,
				getTrackIds: () => player.getTrackIds(),
				updateMusicVolumes: (tracks) => player.updateMusicVolumes(tracks)
			});
			if (cleanup) cleanups.push(cleanup);
		}

		if (cfg.events.audio) {
			const cleanup = handleSampleEvent(node, cfg.events.audio, {
				triggerSample: (sample) => player.triggerSample(sample)
			});
			if (cleanup) cleanups.push(cleanup);
		}
	}

	setup(config);

	return {
		update: setup,
		destroy: () => cleanups.forEach((fn) => fn())
	};
};
