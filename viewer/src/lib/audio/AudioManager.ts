import type { AudioConfiguration } from '@scutoons/content';
import type { IAdaptiveMusicPlayer, IAudioManager, ISampleTriggerManager } from "./types.ts";
import { Howler } from 'howler';
import { SampleTriggerManager } from './SampleTriggerManager.ts';
import { AdaptiveMusicPlayer } from './AdaptiveMusicPlayer.ts';

/**
 * AudioManager - Master coordinator for all audio systems
 * Manages Howler.js global settings and coordinates music/sample systems
 */
export class AudioManager implements IAudioManager {
	musicPlayer: IAdaptiveMusicPlayer | null;
	sampleManager: ISampleTriggerManager | null;
	masterVolume: number;
	isMuted: boolean;
	isInitialized: boolean;
  constructor() {
    this.musicPlayer = null;
    this.sampleManager = null;
    this.masterVolume = 1.0;
    this.isMuted = false;
    this.isInitialized = false;
  }

  async init(audioConfig: AudioConfiguration) {
    if (this.isInitialized)return;
    Howler.volume(this.masterVolume);
    Howler.autoUnlock = true;
    if (audioConfig?.music?.tracks) {
      this.musicPlayer = new AdaptiveMusicPlayer(this);
      await this.musicPlayer.loadTracks(audioConfig.music);
    }
    if (audioConfig?.sampleLibrary && audioConfig.sampleLibrary.length > 0) {
      this.sampleManager = new SampleTriggerManager(this);
      await this.sampleManager.loadSampleLibrary(audioConfig.sampleLibrary);
    }
    this.isInitialized = true;
  }

  setMasterVolume(volume: number): void {
    this.masterVolume = Math.max(0, Math.min(1, volume));
    Howler.volume(this.masterVolume);
  }

  getMasterVolume(): number {
    return this.masterVolume;
  }

  mute(): void {
    this.isMuted = true;
    Howler.mute(true);
  }

  unmute(): void {
    this.isMuted = false;
    Howler.mute(false);
  }

  toggleMute(): void {
    if (this.isMuted) {
      this.unmute();
    } else {
      this.mute();
    }
  }

  isMutedState(): boolean {
    return this.isMuted;
  }

  getMusicPlayer(): IAdaptiveMusicPlayer | null {
    return this.musicPlayer;
  }

  getSampleManager(): ISampleTriggerManager | null {
    return this.sampleManager;
  }

  isAudioUnlocked(): boolean {
    return Howler.ctx && Howler.ctx.state === 'running';
  }

  async cleanup(fadeDuration?: number): Promise<void> {
    const cleanupPromises = [];
    if (this.musicPlayer) {
      cleanupPromises.push(
        this.musicPlayer.fadeOutAndStop(fadeDuration).then(() => {
          this.musicPlayer = null;
        })
      );
    }

    if (this.sampleManager) {
      // Samples use shorter fade (500ms) unless specified
      const sampleFadeDuration = fadeDuration ? Math.min(fadeDuration, 1000) : 500;
      cleanupPromises.push(
        this.sampleManager.cleanup(sampleFadeDuration).then(() => {
          this.sampleManager = null;
        })
      );
    }

    await Promise.all(cleanupPromises);
    Howler.unload();
    this.isInitialized = false;
  }
}
