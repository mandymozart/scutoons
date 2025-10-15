/**
 * ChapterAudio - Audio system initialization and management for chapters
 */
import { AudioManager } from '../../audio/index.js';

export async function initChapterAudio(chapterData) {
  try {
    console.log('Initializing audio system...');

    // Create and initialize AudioManager
    const audioManager = new AudioManager();
    await audioManager.init(chapterData.audio);

    // Start music if available
    const musicPlayer = audioManager.getMusicPlayer();
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

    return audioManager;

  } catch (error) {
    console.error('Failed to initialize audio system:', error);
    return null;
  }
}

/**
 * Find the most recent music configuration by looking backward through slices
 * Each track parameter is found independently (sparse updates per-parameter)
 * @param {Array} slices - All chapter slices
 * @param {number} currentIndex - Current slice index
 * @param {Array} allTrackIds - All available track IDs
 * @returns {Object|null} Music configuration or null
 */
function findPreviousMusicConfig(slices, currentIndex, allTrackIds) {
  // Map<trackId, Map<parameterName, value>>
  const trackParameters = new Map();
  
  // Initialize structure for each track
  allTrackIds.forEach(trackId => {
    trackParameters.set(trackId, new Map());
  });

  let latestTransitionDuration = null;
  const allParameterNames = ['volume']; // Future: 'reverb', 'delay', etc.
  
  // Calculate total parameters to find (tracks × parameters)
  const totalParametersToFind = allTrackIds.length * allParameterNames.length;
  let parametersFound = 0;

  // Look backward from current slice to find most recent value for each parameter
  for (let i = currentIndex - 1; i >= 0; i--) {
    const musicConfig = slices[i]?.audio?.music;
    if (!musicConfig?.tracks) continue;

    // Check each track in this slice's config
    musicConfig.tracks.forEach(track => {
      const trackParams = trackParameters.get(track.id);
      if (!trackParams) return; // Unknown track ID

      // Check each parameter in the track
      allParameterNames.forEach(paramName => {
        // Only store if we haven't found this parameter for this track yet
        if (!trackParams.has(paramName) && track[paramName] !== undefined) {
          trackParams.set(paramName, track[paramName]);
          parametersFound++;
        }
      });
    });

    // Use the transition duration from the most recent slice
    if (latestTransitionDuration === null && musicConfig.transitionDuration !== undefined) {
      latestTransitionDuration = musicConfig.transitionDuration;
    }

    // Stop early if we've found all parameters for all tracks
    if (parametersFound >= totalParametersToFind) {
      break;
    }
  }

  // Build the tracks array from found parameters
  const tracks = [];
  trackParameters.forEach((params, trackId) => {
    if (params.size > 0) {
      // Build track object from found parameters
      const trackObj = { id: trackId };
      params.forEach((value, paramName) => {
        trackObj[paramName] = value;
      });
      tracks.push(trackObj);
    }
  });

  // If we found any parameters, return the config
  if (tracks.length > 0) {
    return {
      tracks,
      transitionDuration: latestTransitionDuration
    };
  }
  
  // No previous config found
  return null;
}

export function setupSliceObserver(audioManager, chapterData) {
  const musicPlayer = audioManager?.getMusicPlayer();
  const sampleManager = audioManager?.getSampleManager();

  if (!musicPlayer && !sampleManager) {
    return null; // No audio systems to manage
  }

  // Track last applied music config to avoid redundant updates
  let lastAppliedConfig = null;

  // Helper to check if two music configs are the same
  function isSameConfig(config1, config2) {
    if (!config1 || !config2) return false;
    if (config1.tracks.length !== config2.tracks.length) return false;
    
    // Create maps for easier comparison
    const map1 = new Map(config1.tracks.map(t => [t.id, t]));
    const map2 = new Map(config2.tracks.map(t => [t.id, t]));
    
    // Check if all tracks have same values
    for (const [id, track1] of map1) {
      const track2 = map2.get(id);
      if (!track2) return false;
      
      // Compare all properties (volume, reverb, etc.)
      for (const key in track1) {
        if (key !== 'id' && track1[key] !== track2[key]) {
          return false;
        }
      }
    }
    
    return true;
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
            // This handles cases where a slice only specifies some tracks
            const allTrackIds = musicPlayer.getTrackIds();
            
            // Start from current slice (include it in the lookup)
            musicConfig = findPreviousMusicConfig(chapterData.slices, sliceIndex + 1, allTrackIds);
            
            // Only apply if config changed
            if (musicConfig && !isSameConfig(musicConfig, lastAppliedConfig)) {
              musicPlayer.updateVolumes(musicConfig.tracks, musicConfig.transitionDuration);
              lastAppliedConfig = musicConfig;
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
