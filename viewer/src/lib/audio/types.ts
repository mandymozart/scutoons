import type { AdoptiveMusicPlayerConfiguration, AudioConfiguration, AudioTrack, Sample } from '@scutoons/content';

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
	fadeOutAndStop(duration?: number): Promise<void>; // equivalent to cleanup(), releases the tracks
	/** releases the tracks (stops their download) */
	unload(): void;
	getTrackIds(): string[];
	/** the music has its own audio output: whether it may play without a user gesture */
	isUnlocked(): boolean;
	/** resume the music output (inside a user gesture) */
	unlock(): void;
	setMasterVolume(volume: number, muted: boolean): void;
}

export interface ISampleTriggerManager {
	loadSampleLibrary(sampleLibrary: AudioTrack[]): Promise<void>;
	trigger(sample: Sample): void;
	cleanup(fadeDuration?: number): Promise<void>;
}