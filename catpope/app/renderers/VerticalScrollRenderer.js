/**
 * @typedef {import('../types').Slice} Slice
 * @typedef {import('../types').Chapter} Chapter
 * @typedef {import('../types').IScutoon} IScutoon
 */

/**
 * VerticalScrollRenderer - Default vertical scrolling chapter renderer
 * Handles presentation layout, delegates behaviors to controllers
 */
import { renderChapterSlice } from '../components/ChapterSlice.js';
import { renderSequenceNavigation } from '../components/SequenceNavigation.js';
import { renderKeyVisual } from '../components/chapter/index.js';
import { AudioBehavior } from '../controllers/behaviors/AudioBehavior.js';
import { HotzoneBehavior } from '../controllers/behaviors/HotzoneBehavior.js';
import { ImageLoadingBehavior } from '../controllers/behaviors/ImageLoadingBehavior.js';
import { KeyVisualBehavior } from '../controllers/behaviors/KeyVisualBehavior.js';
import { scrollToTop, scrollToLabel } from '../utils/scroll.js';

export class VerticalScrollRenderer {
  /**
   * @param {IScutoon} scutoon - The main Scutoon app instance
   */
  constructor(scutoon) {
    this.scutoon = scutoon;
    
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
    
    // If startIndex specified, slice from that point
    const slicesToRender = startIndex > 0 
      ? renderableSlices.slice(startIndex)
      : renderableSlices;

    // Render key visual (only if chapter data provided)
    const keyVisualHtml = chapter?.keyvisual ? renderKeyVisual(chapter.keyvisual, chapter.title) : '';

    // Render slices
    let slicesHtml = '';
    if (slicesToRender && slicesToRender.length > 0) {
      slicesHtml = slicesToRender.map((slice, index) => renderChapterSlice(slice, index)).join('');
    }

    // TODO: Render sequence navigation if needed (renderer can load sequence items)
    const navigationHtml = '';

    // Assemble full HTML
    const content = `
      <div class="chapter-view">
        <div class="webtoon-container">
          ${keyVisualHtml}
          ${slicesHtml}
          ${navigationHtml}
        </div>
      </div>
    `;

    // Insert into DOM
    if (this.scutoon.contentEl) {
      this.scutoon.contentEl.innerHTML = content;
      this.scutoon.contentEl.classList.add('chapter-mode');
      this.scutoon.contentEl.setAttribute('data-renderer', 'vertical');
    }

    // Initialize behaviors
    await this.initBehaviors(chapter, slicesToRender, startIndex);
  }

  /**
   * Initialize all behaviors (audio, hotzones, image loading, etc.)
   * @param {Chapter} chapter - Chapter data
   * @param {Slice[]} slices - Array of slices to initialize
   * @param {number} startIndex - Starting slice index for scrolling
   * @returns {Promise<void>}
   */
  async initBehaviors(chapter, slices, startIndex) {
    // Initialize key visual video (only if provided)
    if (chapter?.keyvisual) {
      this.keyVisualBehavior.init(chapter.keyvisual);
    }

    // Initialize audio (only if chapter provided)
    if (chapter) {
      await this.audioBehavior.init(chapter);
    }

    // Initialize hotzones
    this.hotzoneBehavior.init();

    // Start image loading
    this.imageLoadingBehavior.init(slices);
    
    // Scroll to label if startIndex specified, otherwise scroll to top
    if (startIndex > 0) {
      // Find the label at startIndex
      const renderableSlices = slices.filter(s => s._type !== 'group');
      const targetSlice = renderableSlices[startIndex];
      if (targetSlice && targetSlice._label) {
        // Wait for DOM to render, then scroll
        setTimeout(() => {
          scrollToLabel(targetSlice._label);
        }, 100);
      }
    } else {
      scrollToTop();
    }
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
