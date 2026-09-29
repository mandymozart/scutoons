import type { MusicEventTrack, Slice } from '@scutoons/content';

export interface MusicHandlerContext {
	index: number;
	slices: Slice[];
	/** track ids of the loaded adaptive music player, null while there is none */
	getTrackIds: () => string[] | null;
	updateMusicVolumes: (tracks: MusicEventTrack[]) => void;
}

function findMostRecentTrackState(
	index: number,
	slices: Slice[],
	trackId: string
): Partial<MusicEventTrack> {
	const trackState: Partial<MusicEventTrack> = { id: trackId };
	const propertiesToFind = ['volume'];

	for (let i = index; i >= 0; i--) {
		const slice = slices[i];
		const musicDef = slice.audio?.music?.tracks?.find((t) => t.id === trackId);

		if (musicDef) {
			if (trackState.volume === undefined && musicDef.volume !== undefined) {
				trackState.volume = musicDef.volume;
			}

			const allFound = propertiesToFind.every((prop) => trackState[prop as keyof MusicEventTrack] !== undefined);
			if (allFound) break;
		}
	}

	return trackState;
}

export function getMusicStateForSlice(
	index: number,
	slices: Slice[],
	trackIds: string[]
): MusicEventTrack[] {
	const tracks: MusicEventTrack[] = [];

	for (const trackId of trackIds) {
		const trackState = findMostRecentTrackState(index, slices, trackId);

		if (trackState.volume !== undefined) {
			tracks.push(trackState as MusicEventTrack);
		}
	}

	return tracks;
}

export function handleMusicEvent(node: HTMLElement, context: MusicHandlerContext): (() => void) | null {
	const updateMusic = () => {
		const trackIds = context.getTrackIds();
		if (!trackIds) return;

		const musicState = getMusicStateForSlice(context.index, context.slices, trackIds);

		if (musicState.length > 0) {
			context.updateMusicVolumes(musicState);
		}
	};

	const observer = new IntersectionObserver(
		([entry]) => {
			if (entry.isIntersecting) {
				updateMusic();
			}
		},
		{
			threshold: 0.5,
			rootMargin: '0px'
		}
	);

	observer.observe(node);

	requestAnimationFrame(() => {
		const rect = node.getBoundingClientRect();
		const isInView = rect.top < window.innerHeight && rect.bottom > 0;
		if (isInView) {
			updateMusic();
		}
	});

	return () => observer.disconnect();
}
