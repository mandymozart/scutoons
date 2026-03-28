/**
 * KeyVisualBehavior - Key visual video controller
 * Handles initialization of key visual video functionality
 */

export class KeyVisualBehavior {
  /**
   * Initialize key visual video if present
   */
  init(keyvisual) {
    if (keyvisual && keyvisual.video && keyvisual.image) {
      this.initKeyVisualVideo();
    }
  }

  /**
   * Initialize key visual video playback and controls
   */
  initKeyVisualVideo() {
    const video = document.getElementById('keyVisualVideo');
    const image = document.getElementById('keyVisualImage');
    const replayBtn = document.getElementById('keyVisualReplay');

    if (!video || !image || !replayBtn) return;

    // Function to play video
    const playVideo = () => {
      image.style.display = 'none';
      replayBtn.style.display = 'none';
      video.style.display = 'block';
      video.currentTime = 0;
      video.play().catch(err => {
        console.error('Error playing video:', err);
        // If autoplay fails, show image with replay button
        showImageWithReplay();
      });
    };

    // Function to show image with replay button
    const showImageWithReplay = () => {
      video.style.display = 'none';
      image.style.display = 'block';
      replayBtn.style.display = 'flex';
    };

    // When video ends, show image with replay button
    video.addEventListener('ended', () => {
      showImageWithReplay();
    });

    // Replay button click handler
    replayBtn.addEventListener('click', () => {
      playVideo();
    });

    // Auto-play video when loaded
    video.addEventListener('canplay', () => {
      playVideo();
    }, { once: true });

    // Start loading the video
    video.load();
  }

  /**
   * No cleanup needed
   */
  cleanup() {
    // Nothing to clean up
  }
}
