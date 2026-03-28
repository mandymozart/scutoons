/**
 * AudioBehavior - Audio controller
 * Handles initialization and cleanup of chapter audio
 */
import { AudioManager } from '../../audio/index.js';

export class AudioBehavior {
  constructor() {
    this.audioManager = null;
    this.sliceObserver = null;
    this.lastAppliedConfig = null;
  }

  /**
   * Initialize audio for chapter
   */
  async init(chapterData) {
    if (!chapterData.audio) {
      return;
    }

    try {
      console.log('Initializing audio system...');

      // Create and initialize AudioManager
      this.audioManager = new AudioManager();
      await this.audioManager.init(chapterData.audio);

      // Start music if available
      const musicPlayer = this.audioManager.getMusicPlayer();
      if (musicPlayer) {
        // Wait for user interaction before playing (autoplay policy)
        const startMusic = () => {
          musicPlayer.start();
          console.log('Music started');
          
          // Set initial volumes
          if (chapterData.slices && chapterData.slices[0]?.audio?.music?.tracks) {
            // Use first slice's volume configuration
            const firstSliceTracks = chapterData.slices[0].audio.music.tracks;
            const transitionDuration = chapterData.slices[0].audio.music.transitionDuration || 0;
            musicPlayer.updateVolumes(firstSliceTracks, transitionDuration);
          } else {
            // No slice config - use default track at full volume, others at 0
            const defaultTrackId = musicPlayer.getDefaultTrackId();
            const allTrackIds = musicPlayer.getTrackIds();
            
            if (defaultTrackId) {
              const initialVolumes = allTrackIds.map(trackId => ({
                id: trackId,
                volume: trackId === defaultTrackId ? 1.0 : 0.0
              }));
              musicPlayer.updateVolumes(initialVolumes, 0);
            }
          }

          // Remove the one-time event listener
          document.removeEventListener('click', startMusic);
          document.removeEventListener('scroll', startMusic);
        };

        // Start on first user interaction
        document.addEventListener('click', startMusic, { once: true });
        document.addEventListener('scroll', startMusic, { once: true });
      }

      // Setup slice observer
      if (chapterData.slices) {
        this.sliceObserver = this.setupSliceObserver(chapterData);
      }

    } catch (error) {
      console.error('Failed to initialize audio system:', error);
    }
  }

  /**
   * Setup IntersectionObserver for slice-based audio triggers
   */
  setupSliceObserver(chapterData) {
    const musicPlayer = this.audioManager?.getMusicPlayer();
    const sampleManager = this.audioManager?.getSampleManager();

    if (!musicPlayer && !sampleManager) {
      return null; // No audio systems to manage
    }

    // Create IntersectionObserver for slice visibility
    const sliceObserver = new IntersectionObserver(
      (entries) => {
        entries.forEach(entry => {
          if (entry.isIntersecting) {
            const sliceElement = entry.target;
            const sliceIndex = parseInt(sliceElement.dataset.slice) - 1;
            const sliceData = chapterData.slices[sliceIndex];

            // Update music volumes
            if (musicPlayer) {
              let musicConfig = null;
              
              // Always use sparse lookup to ensure all tracks have values
              const allTrackIds = musicPlayer.getTrackIds();
              
              // Start from current slice (include it in the lookup)
              musicConfig = this.findPreviousMusicConfig(chapterData.slices, sliceIndex + 1, allTrackIds);
              
              // Only apply if config changed
              if (musicConfig && !this.isSameConfig(musicConfig, this.lastAppliedConfig)) {
                musicPlayer.updateVolumes(musicConfig.tracks, musicConfig.transitionDuration);
                this.lastAppliedConfig = musicConfig;
              }
            }

            // Register sample triggers if available
            if (sampleManager && sliceData?.audio?.samples) {
              sampleManager.registerSliceTriggers(sliceElement, sliceData.audio.samples);
            }
          }
        });
      },
      {
        threshold: 0.5, // Trigger when 50% visible
        rootMargin: '0px'
      }
    );

    // Observe all slice elements
    const sliceElements = document.querySelectorAll('.webtoon-slice');
    sliceElements.forEach(slice => {
      sliceObserver.observe(slice);
    });

    console.log(`Observing ${sliceElements.length} slices for audio triggers`);

    return sliceObserver;
  }

  /**
   * Find the most recent music configuration by looking backward through slices
   */
  findPreviousMusicConfig(slices, currentIndex, allTrackIds) {
    const trackParameters = new Map();
    
    // Initialize structure for each track
    allTrackIds.forEach(trackId => {
      trackParameters.set(trackId, new Map());
    });

    let latestTransitionDuration = null;
    const allParameterNames = ['volume'];
    
    const totalParametersToFind = allTrackIds.length * allParameterNames.length;
    let parametersFound = 0;

    // Look backward from current slice
    for (let i = currentIndex - 1; i >= 0; i--) {
      const musicConfig = slices[i]?.audio?.music;
      if (!musicConfig?.tracks) continue;

      musicConfig.tracks.forEach(track => {
        const trackParams = trackParameters.get(track.id);
        if (!trackParams) return;

        allParameterNames.forEach(paramName => {
          if (!trackParams.has(paramName) && track[paramName] !== undefined) {
            trackParams.set(paramName, track[paramName]);
            parametersFound++;
          }
        });
      });

      if (latestTransitionDuration === null && musicConfig.transitionDuration !== undefined) {
        latestTransitionDuration = musicConfig.transitionDuration;
      }

      if (parametersFound >= totalParametersToFind) {
        break;
      }
    }

    // Build tracks array
    const tracks = [];
    trackParameters.forEach((params, trackId) => {
      if (params.size > 0) {
        const trackObj = { id: trackId };
        params.forEach((value, paramName) => {
          trackObj[paramName] = value;
        });
        tracks.push(trackObj);
      }
    });

    if (tracks.length > 0) {
      return {
        tracks,
        transitionDuration: latestTransitionDuration
      };
    }
    
    return null;
  }

  /**
   * Check if two music configs are the same
   */
  isSameConfig(config1, config2) {
    if (!config1 || !config2) return false;
    if (config1.tracks.length !== config2.tracks.length) return false;
    
    const map1 = new Map(config1.tracks.map(t => [t.id, t]));
    const map2 = new Map(config2.tracks.map(t => [t.id, t]));
    
    for (const [id, track1] of map1) {
      const track2 = map2.get(id);
      if (!track2) return false;
      
      for (const key in track1) {
        if (key !== 'id' && track1[key] !== track2[key]) {
          return false;
        }
      }
    }
    
    return true;
  }

  /**
   * Cleanup audio
   */
  async cleanup() {
    // Start audio fade out in background
    if (this.audioManager) {
      this.audioManager.cleanup();
      this.audioManager = null;
    }

    // Cleanup slice observer
    if (this.sliceObserver) {
      this.sliceObserver.disconnect();
      this.sliceObserver = null;
    }
    
    this.lastAppliedConfig = null;
  }
}
