import type { AdoptiveMusicPlayerConfiguration, AudioTrack } from '$lib/models/audio.ts';
import type { IAdaptiveMusicPlayer, IAudioManager } from './types.ts';
import { Howl } from 'howler';

/**
 * AdaptiveMusicPlayer - 4-parallel-track emotion-based music system
 * Manages smooth crossfading between emotional states based on slice data
 */
export class AdaptiveMusicPlayer implements IAdaptiveMusicPlayer {
  audioManager: IAudioManager;
	tracks: Map<string, Howl>;
	currentVolumes: Map<string, number>;
	isPlaying: boolean;
	defaultTransitionDuration: number;
	defaultTrackId: string | null;
	constructor(audioManager: IAudioManager) {
    this.audioManager = audioManager;
    this.tracks = new Map(); // Map<trackId, Howl>
    this.currentVolumes = new Map(); // Map<trackId, number>
    this.isPlaying = false;
    this.defaultTransitionDuration = 3000; // ms
    this.defaultTrackId = null; // Default track to play at full volume
  }

  async loadTracks(musicConfig: AdoptiveMusicPlayerConfiguration) {
    if (!musicConfig || !musicConfig.tracks || musicConfig.tracks.length === 0) return;
    
		this.defaultTransitionDuration = musicConfig.transitionDuration || 3000;

    if (musicConfig.defaultTrack) {
      this.defaultTrackId = musicConfig.defaultTrack;
    } else if (musicConfig.tracks.length > 0) {
      this.defaultTrackId = musicConfig.tracks[0].id;
    }

    const loadPromises = musicConfig.tracks.map(trackConfig => 
      this.loadTrack(trackConfig)
    );

    await Promise.all(loadPromises);
    console.log(`Loaded ${this.tracks.size} music tracks (default: ${this.defaultTrackId})`);
  }

  loadTrack(trackConfig: AudioTrack): Promise<void> {
    return new Promise((resolve, reject) => {
      const howl = new Howl({
        src: [trackConfig.path],
        loop: true,
        volume: 0, // Start at 0, will be adjusted by slice
        preload: true,
        html5: false, // Use Web Audio API for better control
        onload: () => {
          console.log(`Track loaded: ${trackConfig.id}`);
          resolve();
        },
        onloaderror: (id, error) => {
          console.error(`Failed to load track: ${trackConfig.id}`, error);
          reject(error);
        }
      });

      this.tracks.set(trackConfig.id, howl);
      this.currentVolumes.set(trackConfig.id, 0);
    });
  }

  start() {
    if (this.isPlaying) return;
    this.tracks.forEach((howl) => {
      howl.play();
    });
    this.isPlaying = true;
  }

  updateVolumes(trackVolumes: {id: string, volume: number}[], duration?: number) {
    if (!this.isPlaying) return;

    const transitionDuration = duration || this.defaultTransitionDuration;
    
    trackVolumes.forEach(trackVolume => {
      const howl = this.tracks.get(trackVolume.id);
      if (!howl) {
        return;
      }

      const targetVolume = Math.max(0, Math.min(1, trackVolume.volume));
      const currentVolume = this.currentVolumes.get(trackVolume.id) || 0;
      howl.fade(currentVolume, targetVolume, transitionDuration);
      this.currentVolumes.set(trackVolume.id, targetVolume);
    });
  }

  pause() {
    if (!this.isPlaying) return;
    this.tracks.forEach(howl => {
      howl.pause();
    });
    this.isPlaying = false;
  }

  resume() {
    if (this.isPlaying) return;
    this.tracks.forEach(howl => {
      howl.play();
    });
    this.isPlaying = true;
  }

  stop() {
    this.tracks.forEach(howl => {
      howl.stop();
    });
    this.isPlaying = false;
  }

  fadeOutAndStop(duration?: number): Promise<void> {
    return new Promise((resolve) => {
      if (!this.isPlaying) {
        resolve();
        return;
      }
      const fadeDuration = duration !== undefined && duration !== null ? duration : this.defaultTransitionDuration;
      this.tracks.forEach((howl, trackId) => {
        const currentVolume = howl.volume();
        if (currentVolume > 0) {
          howl.fade(currentVolume, 0, fadeDuration);
          this.currentVolumes.set(trackId, 0);
        }
      });

      setTimeout(() => {
        this.stop();
        resolve();
      }, fadeDuration);
    });
  }

  getCurrentVolumes(): Map<string, number> {
    return new Map(this.currentVolumes);
  }

  isCurrentlyPlaying(): boolean {
    return this.isPlaying;
  }

  getDefaultTrackId(): string | null {
    return this.defaultTrackId;
  }

  getTrackIds(): string[] {
    return Array.from(this.tracks.keys());
  }
}
