import { Howl } from 'howler';
import { TriggerMode } from '@scutoons/content';
import type { Sample, AudioTrack } from '@scutoons/content';
/**
 * SampleTriggerManager - Event-based audio sample playback
 * Handles loading, caching, and triggering audio samples based on user interaction.
 * Samples load on demand (agents/research/LOADING.md L2): the first slices' at the start, the others when their slice
 * comes near (`want`) or – a fast scroll – at their first trigger, played as soon as they are there.
 */
import type { IAudioManager, ISampleTriggerManager } from './types.ts';

/** a trigger that waited for its file longer than this is dropped (the sound would come out of place) */
const LATE_MS = 1000;

export class SampleTriggerManager implements ISampleTriggerManager {
  audioManager: IAudioManager;
  sampleLibrary: Map<string, Howl>;
  /** the chapter's samples: id → config (fetched or not) */
  private known = new Map<string, AudioTrack>();
  /** triggered before its file was there */
  private waiting = new Map<string, { config: Sample; at: number }>();

  constructor(audioManager: IAudioManager) {
    this.audioManager = audioManager;
    this.sampleLibrary = new Map(); // Map<sampleId, Howl>
  }

  /** `eager`: the ids to fetch now (null = all, the `generous` data tier); the others wait for `want` or a trigger */
  async loadSampleLibrary(sampleLibrary: AudioTrack[], eager: string[] | null = null) {
    if (!sampleLibrary || sampleLibrary.length === 0) return;
    for (const sample of sampleLibrary) this.known.set(sample.id, sample);
    const now = sampleLibrary.filter((sample) => eager === null || eager.includes(sample.id));
    // a failing sample doesn't hold up the chapter's audio
    await Promise.all(now.map((sample) => this.loadSample(sample).catch(() => {})));
    console.log(`Loaded ${now.length} of ${sampleLibrary.length} samples`);
  }

  /** a slice with these samples is near: fetch them */
  want(ids: string[]) {
    for (const id of ids) {
      const config = this.known.get(id);
      if (config && !this.sampleLibrary.has(id)) void this.loadSample(config).catch(() => {});
    }
  }

  loadSample(sampleConfig: AudioTrack): Promise<void> {
    return new Promise((resolve, reject) => {
      const howl = new Howl({
        src: [sampleConfig.path],
        preload: true,
        html5: false,
        onload: () => {
          resolve();
          // triggered while it was loading: play it now, unless that was too long ago
          const waiting = this.waiting.get(sampleConfig.id);
          this.waiting.delete(sampleConfig.id);
          if (waiting && performance.now() - waiting.at < LATE_MS) this.trigger(waiting.config);
        },
        onloaderror: (id, error) => {
          console.error(`Failed to load sample: ${sampleConfig.id}`, error);
          this.waiting.delete(sampleConfig.id);
          reject(error);
        }
      });

      this.sampleLibrary.set(sampleConfig.id, howl);
    });
  }

  trigger(config: Sample) {
    const howl = this.sampleLibrary.get(config.id);
    if (!howl || howl.state() !== 'loaded') {
      // not there yet (its slice came too fast): fetch it and play it as soon as it is
      if (!this.known.has(config.id)) return void console.warn(`Sample not found: ${config.id}`);
      this.waiting.set(config.id, { config, at: performance.now() });
      if (!howl) this.want([config.id]);
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
  }

  async cleanup(fadeDuration = 500): Promise<void> {
    // Fade out and stop any playing samples
    const playingSamples = [];
    this.sampleLibrary.forEach((howl) => {
      if (howl.playing()) {
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
    this.known.clear();
    this.waiting.clear();
  }
}
