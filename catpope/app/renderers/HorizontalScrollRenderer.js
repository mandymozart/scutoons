/**
 * HorizontalScrollRenderer - Horizontal pager/swiper chapter renderer
 * Handles presentation layout, delegates behaviors to controllers
 * TODO: Implement horizontal layout with paging/swiping
 * @typedef {import('../types').Slice} Slice
 * @typedef {import('../types').Chapter} Chapter
 * @typedef {import('../types').Scutoon} Scutoon
 */
import { renderChapterSlice } from '../components/ChapterSlice.js';
import { renderSequenceNavigation } from '../components/SequenceNavigation.js';
import { renderKeyVisual } from '../components/chapter/index.js';
import { AudioBehavior } from '../controllers/behaviors/AudioBehavior.js';
import { HotzoneBehavior } from '../controllers/behaviors/HotzoneBehavior.js';
import { ImageLoadingBehavior } from '../controllers/behaviors/ImageLoadingBehavior.js';
import { KeyVisualBehavior } from '../controllers/behaviors/KeyVisualBehavior.js';
import { scrollToTop } from '../utils/scroll.js';

export class HorizontalScrollRenderer {
  /**
   * @param {Scutoon} scutoon - The main Scutoon app instance
   */
  constructor(scutoon) {
    this.scutoon = scutoon;
    this.currentPage = 0;
    
    // Behavior controllers
    this.audioBehavior = new AudioBehavior();
    this.hotzoneBehavior = new HotzoneBehavior();
    this.imageLoadingBehavior = new ImageLoadingBehavior();
    this.keyVisualBehavior = new KeyVisualBehavior();
  }

  /**
   * @param {import('../types').RenderOptions} options
   * @returns {Promise<void>}
   */
  async render({ slices = [], chapter, startIndex = 0 }) {
    // Filter out virtual group markers
    const renderableSlices = slices.filter(slice => slice._type !== 'group');
    
    // If startIndex specified, start from that page
    this.currentPage = startIndex;

    // Render key visual (only if chapter data provided)
    const keyVisualHtml = chapter?.keyvisual ? renderKeyVisual(chapter.keyvisual, chapter.title) : '';

    // Render slices (same structure as vertical, different scrolling)
    let slicesHtml = '';
    if (renderableSlices && renderableSlices.length > 0) {
      slicesHtml = renderableSlices.map((slice, index) => 
        renderChapterSlice(slice, index)
      ).join('');
    }

    // TODO: Render sequence navigation if needed (renderer can load sequence items)
    const navigationHtml = '';

    // Assemble full HTML with horizontal layout
    const content = `
      <div class="chapter-view horizontal-scroll">
        ${keyVisualHtml}
        <div class="slices horizontal-slices">
          ${slicesHtml}
        </div>
        <div class="page-controls">
          <button class="prev-page">←</button>
          <span class="page-indicator">${this.currentPage + 1} / ${renderableSlices.length}</span>
          <button class="next-page">→</button>
        </div>
        ${navigationHtml}
      </div>
    `;

    // Insert into DOM
    if (this.scutoon.contentEl) {
      this.scutoon.contentEl.innerHTML = content;
      this.scutoon.contentEl.classList.add('chapter-mode', 'horizontal-mode');
      this.scutoon.contentEl.setAttribute('data-renderer', 'horizontal');
    }

    // Initialize behaviors
    await this.initBehaviors(chapter, renderableSlices);
    
    // TODO: Setup page navigation event listeners
    this.setupPageNavigation(renderableSlices.length);
  }

  /**
   * Setup page navigation controls (placeholder)
   */
  setupPageNavigation(totalPages) {
    // TODO: Implement swipe gestures, arrow keys, button clicks
    console.log('TODO: Setup page navigation for', totalPages, 'pages');
  }

  /**
   * @param {Chapter} chapter
   * @param {Slice[]} slices
   * @returns {Promise<void>}
   */
  async initBehaviors(chapter, slices) {
    // Initialize key visual video (only if provided)
    if (chapter?.keyvisual) {
      this.keyVisualBehavior.init(chapter.keyvisual);
    }
    
    // Initialize audio (only if chapter provided)
    if (chapter) {
      await this.audioBehavior.init(chapter);
    }
    this.hotzoneBehavior.init();
    this.imageLoadingBehavior.init(slices);
    scrollToTop();
  }

  /**
   * Cleanup all behaviors
   */
  async cleanup() {
    await this.audioBehavior.cleanup();
    this.hotzoneBehavior.destroy();
    this.imageLoadingBehavior.cleanup();
    this.keyVisualBehavior.cleanup();
  }
}
