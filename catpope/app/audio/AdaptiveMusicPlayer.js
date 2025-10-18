/**
 * AdaptiveMusicPlayer - 4-parallel-track emotion-based music system
 * Manages smooth crossfading between emotional states based on slice data
 */
import { asset } from '../utils/cdn.js';

export class AdaptiveMusicPlayer {
  constructor(audioManager) {
    this.audioManager = audioManager;
    this.tracks = new Map(); // Map<trackId, Howl>
    this.currentVolumes = new Map(); // Map<trackId, number>
    this.isPlaying = false;
    this.defaultTransitionDuration = 3000; // ms
    this.defaultTrackId = null; // Default track to play at full volume
  }

  /**
   * Load music tracks from config
   * @param {Object} musicConfig - Music configuration from chapter
   */
  async loadTracks(musicConfig) {
    console.log('Loading music tracks...');

    if (!musicConfig || !musicConfig.tracks || musicConfig.tracks.length === 0) {
      console.warn('No music tracks to load');
      return;
    }

    this.defaultTransitionDuration = musicConfig.transitionDuration || 3000;

    // Determine default track
    if (musicConfig.defaultTrack) {
      // Use specified default track
      this.defaultTrackId = musicConfig.defaultTrack;
    } else if (musicConfig.tracks.length > 0) {
      // Use first track as default if not specified
      this.defaultTrackId = musicConfig.tracks[0].id;
    }

    // Load all tracks
    const loadPromises = musicConfig.tracks.map(trackConfig => 
      this.loadTrack(trackConfig)
    );

    await Promise.all(loadPromises);
    console.log(`Loaded ${this.tracks.size} music tracks (default: ${this.defaultTrackId})`);
  }

  /**
   * Load a single track
   * @param {Object} trackConfig - Track configuration {id, path}
   */
  loadTrack(trackConfig) {
    return new Promise((resolve, reject) => {
      const howl = new Howl({
        src: [asset(trackConfig.path)],
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

  /**
   * Start playing all tracks
   */
  start() {
    if (this.isPlaying) {
      console.warn('Music already playing');
      return;
    }

    console.log('Starting music playback...');

    // Start all tracks at volume 0
    this.tracks.forEach((howl, trackId) => {
      howl.play();
    });

    this.isPlaying = true;
    console.log('Music playback started');
  }

  /**
   * Update track volumes with crossfade
   * @param {Array} trackVolumes - Array of {id, volume} objects
   * @param {number} duration - Crossfade duration in ms (optional)
   */
  updateVolumes(trackVolumes, duration) {
    if (!this.isPlaying) {
      console.warn('Music not playing, cannot update volumes');
      return;
    }

    const transitionDuration = duration || this.defaultTransitionDuration;
    
    console.log('Updating track volumes:', trackVolumes);

    trackVolumes.forEach(trackVolume => {
      const howl = this.tracks.get(trackVolume.id);
      if (!howl) {
        console.warn(`Track not found: ${trackVolume.id}`);
        return;
      }

      const targetVolume = Math.max(0, Math.min(1, trackVolume.volume));
      const currentVolume = this.currentVolumes.get(trackVolume.id) || 0;

      // Use Howler's fade method for smooth transitions
      howl.fade(currentVolume, targetVolume, transitionDuration);
      
      // Update stored volume
      this.currentVolumes.set(trackVolume.id, targetVolume);
    });
  }

  /**
   * Pause all music tracks
   */
  pause() {
    if (!this.isPlaying) return;

    this.tracks.forEach(howl => {
      howl.pause();
    });

    this.isPlaying = false;
    console.log('Music paused');
  }

  /**
   * Resume music playback
   */
  resume() {
    if (this.isPlaying) return;

    this.tracks.forEach(howl => {
      howl.play();
    });

    this.isPlaying = true;
    console.log('Music resumed');
  }

  /**
   * Stop all music tracks
   */
  stop() {
    this.tracks.forEach(howl => {
      howl.stop();
    });

    this.isPlaying = false;
    console.log('Music stopped');
  }

  /**
   * Fade out all tracks and then stop
   * @param {number} duration - Fade out duration in ms (optional)
   * @returns {Promise} Resolves when fade out completes
   */
  fadeOutAndStop(duration) {
    return new Promise((resolve) => {
      if (!this.isPlaying) {
        resolve();
        return;
      }

      const fadeDuration = duration !== undefined && duration !== null ? duration : this.defaultTransitionDuration;
      
      console.log(`Fading out music over ${fadeDuration}ms...`);

      // Fade all tracks to 0
      const fadePromises = [];
      this.tracks.forEach((howl, trackId) => {
        // Get actual current volume from Howler
        const currentVolume = howl.volume();
        if (currentVolume > 0) {
          howl.fade(currentVolume, 0, fadeDuration);
          this.currentVolumes.set(trackId, 0);
        }
      });

      // Stop after fade completes
      setTimeout(() => {
        this.stop();
        resolve();
      }, fadeDuration);
    });
  }

  /**
   * Get current volume levels
   * @returns {Map<string, number>} Map of track IDs to volumes
   */
  getCurrentVolumes() {
    return new Map(this.currentVolumes);
  }

  /**
   * Check if music is currently playing
   * @returns {boolean}
   */
  isCurrentlyPlaying() {
    return this.isPlaying;
  }

  /**
   * Get the default track ID
   * @returns {string|null}
   */
  getDefaultTrackId() {
    return this.defaultTrackId;
  }

  /**
   * Get all track IDs
   * @returns {Array<string>}
   */
  getTrackIds() {
    return Array.from(this.tracks.keys());
  }
}
