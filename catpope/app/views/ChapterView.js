/**
 * ChapterView - Webtoon chapter reader
 */
import { loadChapter } from '../utils/api.js';
import { loadImagesSequentially } from '../utils/imageLoader.js';
import { renderChapterSlice } from '../components/ChapterSlice.js';
import { MembershipRequiredView } from './MembershipRequiredView.js';
import { 
  renderKeyVisual, 
  initKeyVisualVideo,
  initChapterAudio,
  setupSliceObserver
} from '../components/chapter/index.js';

export class ChapterView {
  constructor(scutoon) {
    this.scutoon = scutoon;
    this.audioManager = null;
    this.sliceObserver = null;
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

    // Render key visual
    const keyVisualHtml = renderKeyVisual(data.keyvisual, data.title);

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
      initKeyVisualVideo();
    }

    // Initialize audio system if chapter has audio config
    if (data.audio) {
      this.audioManager = await initChapterAudio(data);
      if (this.audioManager) {
        this.sliceObserver = setupSliceObserver(this.audioManager, data);
      }
    }

    // Start sequential lazy loading
    if (data.slices && data.slices.length > 0) {
      loadImagesSequentially();
    }

    return data;
  }

  cleanup() {
    // Cleanup audio system
    if (this.audioManager) {
      this.audioManager.cleanup();
      this.audioManager = null;
    }

    // Cleanup slice observer
    if (this.sliceObserver) {
      this.sliceObserver.disconnect();
      this.sliceObserver = null;
    }

    console.log('ChapterView cleaned up');
  }
}
