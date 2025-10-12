/**
 * ChapterView - Webtoon chapter reader
 */
import { loadChapter } from '../utils/api.js';
import { loadImagesSequentially } from '../utils/imageLoader.js';
import { renderChapterSlice } from '../components/ChapterSlice.js';
import { MembershipRequiredView } from './MembershipRequiredView.js';

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

    const data = await loadChapter(this.scutoon.toonName, this.scutoon.lang, chapterNumber);
    this.scutoon.chapter = data;
    this.scutoon.navigationStore.setView('chapter', { chapter: chapterNumber });

    // Update URL
    this.scutoon.navigationStore.updateURL({ view: 'chapter', chapter: chapterNumber });

    // Create placeholder HTML for images
    let slicesHtml = '';
    if (data.slices && data.slices.length > 0) {
      slicesHtml = data.slices.map((slice, index) => renderChapterSlice(slice, index)).join('');
    }

    const content = `
      <div class="chapter-view">
        <div class="webtoon-container">
          ${slicesHtml}
        </div>
      </div>
    `;

    if (this.scutoon.contentEl) {
      this.scutoon.contentEl.innerHTML = content;
      // Make content div fullscreen for chapter view
      this.scutoon.contentEl.classList.add('chapter-mode');
    }

    // Start sequential lazy loading
    if (data.slices && data.slices.length > 0) {
      loadImagesSequentially();
    }

    return data;
  }
}
