import type { AdoptiveMusicPlayerConfiguration, AudioConfiguration, AudioTrack, Sample } from "$lib/models/audio.ts";

export interface IAudioManager {
	init(audioConfig: AudioConfiguration): Promise<void>;
	setMasterVolume(volume: number): void;
	getMasterVolume(): number;
	toggleMute(): void;
	getMusicPlayer(): IAdaptiveMusicPlayer | null;
	getSampleManager(): ISampleTriggerManager | null;
	cleanup(fadeDuration?: number): Promise<void>;
}

export interface IAdaptiveMusicPlayer {
	loadTracks(musicConfig: AdoptiveMusicPlayerConfiguration): Promise<void>;
	start(): void;
	pause(): void;
	resume(): void;
	stop(): void;
	updateVolumes(volumes: { id: string; volume: number }[], duration?: number): void;
	isCurrentlyPlaying(): boolean;
	fadeOutAndStop(duration?: number): Promise<void>; // equivalent to cleanup()
}

export interface ISampleTriggerManager {
	loadSampleLibrary(sampleLibrary: AudioTrack[]): Promise<void>;
	trigger(sample: Sample): void;
	cleanup(fadeDuration?: number): Promise<void>;
}