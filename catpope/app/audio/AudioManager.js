/**
 * AudioManager - Master coordinator for all audio systems
 * Manages Howler.js global settings and coordinates music/sample systems
 */

export class AudioManager {
  constructor() {
    this.musicPlayer = null;
    this.sampleManager = null;
    this.masterVolume = 1.0;
    this.isMuted = false;
    this.isInitialized = false;
  }

  /**
   * Initialize the audio system with chapter audio config
   * @param {Object} audioConfig - Chapter-level audio configuration
   */
  async init(audioConfig) {
    if (this.isInitialized) {
      console.warn('AudioManager already initialized');
      return;
    }

    // Check if Howler is available
    if (typeof Howler === 'undefined') {
      console.error('Howler.js not loaded. Audio system disabled.');
      return;
    }

    console.log('Initializing AudioManager...');

    // Set Howler global settings
    Howler.volume(this.masterVolume);
    Howler.autoUnlock = true; // Automatically unlock audio on user interaction

    // Initialize music player if music config exists
    if (audioConfig?.music?.tracks) {
      const { AdaptiveMusicPlayer } = await import('./AdaptiveMusicPlayer.js');
      this.musicPlayer = new AdaptiveMusicPlayer(this);
      await this.musicPlayer.loadTracks(audioConfig.music);
      console.log('Adaptive Music Player initialized');
    }

    // Initialize sample manager if sample library exists
    if (audioConfig?.sampleLibrary && audioConfig.sampleLibrary.length > 0) {
      const { SampleTriggerManager } = await import('./SampleTriggerManager.js');
      this.sampleManager = new SampleTriggerManager(this);
      await this.sampleManager.loadSampleLibrary(audioConfig.sampleLibrary);
      console.log('Sample Trigger Manager initialized');
    }

    this.isInitialized = true;
    console.log('AudioManager initialized successfully');
  }

  /**
   * Set master volume for all audio
   * @param {number} volume - Volume level (0.0 to 1.0)
   */
  setMasterVolume(volume) {
    this.masterVolume = Math.max(0, Math.min(1, volume));
    if (typeof Howler !== 'undefined') {
      Howler.volume(this.masterVolume);
    }
    console.log(`Master volume set to ${this.masterVolume}`);
  }

  /**
   * Get current master volume
   * @returns {number} Current volume (0.0 to 1.0)
   */
  getMasterVolume() {
    return this.masterVolume;
  }

  /**
   * Mute all audio
   */
  mute() {
    this.isMuted = true;
    if (typeof Howler !== 'undefined') {
      Howler.mute(true);
    }
    console.log('Audio muted');
  }

  /**
   * Unmute all audio
   */
  unmute() {
    this.isMuted = false;
    if (typeof Howler !== 'undefined') {
      Howler.mute(false);
    }
    console.log('Audio unmuted');
  }

  /**
   * Toggle mute state
   */
  toggleMute() {
    if (this.isMuted) {
      this.unmute();
    } else {
      this.mute();
    }
  }

  /**
   * Check if audio is muted
   * @returns {boolean}
   */
  isMutedState() {
    return this.isMuted;
  }

  /**
   * Get the music player instance
   * @returns {AdaptiveMusicPlayer|null}
   */
  getMusicPlayer() {
    return this.musicPlayer;
  }

  /**
   * Get the sample manager instance
   * @returns {SampleTriggerManager|null}
   */
  getSampleManager() {
    return this.sampleManager;
  }

  /**
   * Check if audio context is unlocked (user has interacted)
   * @returns {boolean}
   */
  isAudioUnlocked() {
    if (typeof Howler === 'undefined') return false;
    return Howler.ctx && Howler.ctx.state === 'running';
  }

  /**
   * Cleanup and stop all audio with fade out
   * @param {number} fadeDuration - Fade out duration in ms (optional)
   * @returns {Promise} Resolves when cleanup is complete
   */
  async cleanup(fadeDuration) {
    console.log('Cleaning up AudioManager...');

    // Fade out both music and samples in parallel
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

    // Wait for all audio to fade out
    await Promise.all(cleanupPromises);

    // Stop and unload all Howler sounds
    if (typeof Howler !== 'undefined') {
      Howler.unload();
    }

    this.isInitialized = false;
    console.log('AudioManager cleaned up');
  }
}
