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

export function setupSliceObserver(audioManager, chapterData) {
  const musicPlayer = audioManager?.getMusicPlayer();
  const sampleManager = audioManager?.getSampleManager();

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

          if (!sliceData?.audio) return;

          // Update music volumes if available
          if (musicPlayer && sliceData.audio.music?.tracks) {
            const transitionDuration = sliceData.audio.music.transitionDuration;
            musicPlayer.updateVolumes(sliceData.audio.music.tracks, transitionDuration);
          }

          // Register sample triggers if available
          if (sampleManager && sliceData.audio.samples) {
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
