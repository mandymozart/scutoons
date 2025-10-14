/**
 * KeyVisual component - renders chapter key visual (image or video)
 */
import { escapeHtml, sanitizeUrl, escapeAttr } from '../../utils/template.js';

export function renderKeyVisual(keyvisualData, title) {
  if (!keyvisualData) return '';

  if (keyvisualData.video && keyvisualData.image) {
    // Both video and image available
    const videoUrl = sanitizeUrl(keyvisualData.video);
    const imageUrl = sanitizeUrl(keyvisualData.image);
    if (videoUrl && imageUrl) {
      return `
        <div class="key-visual-container">
          <img src="${escapeHtml(imageUrl)}" alt="${escapeAttr(title)}" class="key-visual key-visual-image" id="keyVisualImage">
          <video src="${escapeHtml(videoUrl)}" class="key-visual key-visual-video" id="keyVisualVideo" muted playsinline style="display: none;"></video>
          <button class="key-visual-replay" id="keyVisualReplay" style="display: none;">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polygon points="5 3 19 12 5 21 5 3"></polygon>
            </svg>
          </button>
        </div>
      `;
    }
  } else if (keyvisualData.image) {
    // Only image available
    const imageUrl = sanitizeUrl(keyvisualData.image);
    if (imageUrl) {
      return `<img src="${escapeHtml(imageUrl)}" alt="${escapeAttr(title)}" class="key-visual">`;
    }
  }

  return '';
}

export function initKeyVisualVideo() {
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
