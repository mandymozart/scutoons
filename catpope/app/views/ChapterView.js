/**
 * ChapterView - Webtoon chapter reader
 */
import { loadChapter } from '../utils/api.js';
import { loadImagesSequentially } from '../utils/imageLoader.js';
import { renderChapterSlice } from '../components/ChapterSlice.js';
import { MembershipRequiredView } from './MembershipRequiredView.js';
import { escapeHtml, sanitizeUrl, escapeAttr } from '../utils/template.js';

export class ChapterView {
  constructor(scutoon) {
    this.scutoon = scutoon;
  }

  async render(chapterNumber = 1) {
    // Check membership before rendering chapter
    if (!this.scutoon.membershipStore.hasValidMembership()) {
      const membershipView = new MembershipRequiredView(this.scutoon);
      membershipView.render(chapterNumber);
      return null;
    }

    const data = await loadChapter(this.scutoon.lang, chapterNumber);
    this.scutoon.chapter = data;
    this.scutoon.navigationStore.setView('chapter', { chapter: chapterNumber });

    // Update URL
    this.scutoon.navigationStore.updateURL({ view: 'chapter', chapter: chapterNumber });

    // Key visual with video support
    let keyVisualHtml = '';
    if (data.keyvisual) {
      if (data.keyvisual.video && data.keyvisual.image) {
        // Both video and image available
        const videoUrl = sanitizeUrl(data.keyvisual.video);
        const imageUrl = sanitizeUrl(data.keyvisual.image);
        if (videoUrl && imageUrl) {
          keyVisualHtml = `
            <div class="key-visual-container">
              <img src="${escapeHtml(imageUrl)}" alt="${escapeAttr(data.title)}" class="key-visual key-visual-image" id="keyVisualImage">
              <video src="${escapeHtml(videoUrl)}" class="key-visual key-visual-video" id="keyVisualVideo" muted playsinline style="display: none;"></video>
              <button class="key-visual-replay" id="keyVisualReplay" style="display: none;">
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polygon points="5 3 19 12 5 21 5 3"></polygon>
                </svg>
              </button>
            </div>
          `;
        }
      } else if (data.keyvisual.image) {
        // Only image available
        const imageUrl = sanitizeUrl(data.keyvisual.image);
        if (imageUrl) {
          keyVisualHtml = `<img src="${escapeHtml(imageUrl)}" alt="${escapeAttr(data.title)}" class="key-visual">`;
        }
      }
    }

    // Create placeholder HTML for images
    let slicesHtml = '';
    if (data.slices && data.slices.length > 0) {
      slicesHtml = data.slices.map((slice, index) => renderChapterSlice(slice, index)).join('');
    }

    const content = `
      <div class="chapter-view">
        <div class="webtoon-container">
          ${keyVisualHtml}
          ${slicesHtml}
        </div>
      </div>
    `;

    if (this.scutoon.contentEl) {
      this.scutoon.contentEl.innerHTML = content;
      // Make content div fullscreen for chapter view
      this.scutoon.contentEl.classList.add('chapter-mode');
    }

    // Initialize key visual video functionality
    if (data.keyvisual && data.keyvisual.video && data.keyvisual.image) {
      this.initKeyVisualVideo();
    }

    // Start sequential lazy loading
    if (data.slices && data.slices.length > 0) {
      loadImagesSequentially();
    }

    return data;
  }

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
}
