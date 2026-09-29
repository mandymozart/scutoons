import { TriggerMode } from '@scutoons/content';
import type { SampleTrigger, Sample, AudioTrack } from '@scutoons/content';
/**
 * SampleTriggerManager - Event-based audio sample playback
 * Handles loading, caching, and triggering audio samples based on user interaction
 */
import type { IAudioManager, ISampleTriggerManager } from './types.ts';

export class SampleTriggerManager implements ISampleTriggerManager {
  audioManager: IAudioManager;
  sampleLibrary: Map<string, Howl>;

  constructor(audioManager: IAudioManager) {
    this.audioManager = audioManager;
    this.sampleLibrary = new Map(); // Map<sampleId, Howl>
  }

  async loadSampleLibrary(sampleLibrary:AudioTrack[]) {
    console.log('Loading sample library...');

    if (!sampleLibrary || sampleLibrary.length === 0) {
      console.warn('No samples to load');
      return;
    }

    // Load all samples
    const loadPromises = sampleLibrary.map(sampleConfig => 
      this.loadSample(sampleConfig)
    );

    await Promise.all(loadPromises);
    console.log(`Loaded ${this.sampleLibrary.size} samples`);
  }

  loadSample(sampleConfig: AudioTrack):Promise<void> {
    return new Promise((resolve, reject) => {
      const howl = new Howl({
        src: [sampleConfig.path],
        preload: true,
        html5: false,
        onload: () => {
          console.log(`Sample loaded: ${sampleConfig.id}`);
          resolve();
        },
        onloaderror: (id, error) => {
          console.error(`Failed to load sample: ${sampleConfig.id}`, error);
          reject(error);
        }
      });

      this.sampleLibrary.set(sampleConfig.id, howl);
    });
  }

  // DOM registration methods removed - now handled by Svelte actions
  // See: src/lib/actions/audio.ts

  trigger(config: Sample) {
    const howl = this.sampleLibrary.get(config.id);
    if (!howl) {
      console.warn(`Sample not found: ${config.id}`);
      return;
    }

    // Handle gating - don't play if already playing
    if (config.trigger?.gate && howl.playing()) {
      return;
    }

    // Handle mono mode - debounce (don't retrigger if already playing)
    if (config.trigger?.mode === TriggerMode.MONO && howl.playing()) {
      return; // Debounce - already playing, skip this trigger
    }

    // Handle poly mode - allow multiple overlapping instances
    // (default Howler behavior, nothing special needed)

    // Handle exclusive mode - mute other audio
    if (config.trigger?.exclusive) {
      // TODO: Implement exclusive mode (mute other samples/music)
      console.log('Exclusive mode not yet implemented');
    }

    // Set sample-specific volume
    if (config.volume !== undefined) {
      howl.volume(config.volume);
    }

    // Set loop
    if (config.loop !== undefined) {
      howl.loop(config.loop);
    }

    // Apply delay if specified
    const delay = config.delay || 0;
    
    if (delay > 0) {
      setTimeout(() => {
        howl.play();
      }, delay);
    } else {
      howl.play();
    }

    console.log(`Triggered sample: ${config.id}${config.trigger?.mode === 'poly' ? ' (poly)' : ''}`);
  }

  async cleanup(fadeDuration = 500): Promise<void> {
    console.log('Cleaning up sample triggers...');

    // Fade out and stop any playing samples
    const playingSamples = [];
    this.sampleLibrary.forEach((howl, sampleId) => {
      if (howl.playing()) {
        console.log(`Fading out sample: ${sampleId}`);
        const currentVolume = howl.volume();
        howl.fade(currentVolume, 0, fadeDuration);
        playingSamples.push(howl);
      }
    });

    // Wait for fade out to complete if there are playing samples
    if (playingSamples.length > 0) {
      await new Promise(resolve => setTimeout(resolve, fadeDuration));
    }

    // Unload all samples
    this.sampleLibrary.forEach((howl) => {
      howl.unload();
    });

    this.sampleLibrary.clear();

    console.log('Sample triggers cleaned up');
  }
}
