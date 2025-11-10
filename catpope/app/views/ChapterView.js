/**
 * ChapterView - Webtoon chapter reader
 */
import { loadChapter, loadSeason, loadSong } from '../utils/api.js';
import { loadImagesSequentially } from '../utils/imageLoader.js';
import { renderChapterSlice } from '../components/ChapterSlice.js';
import { renderSequenceNavigation } from '../components/SequenceNavigation.js';
import { MembershipRequiredView } from './MembershipRequiredView.js';
import { sanitizeNumber } from '../utils/template.js';
import { 
  renderKeyVisual, 
  initKeyVisualVideo,
  initChapterAudio,
  setupSliceObserver,
  initHotzoneVisualizer
} from '../components/chapter/index.js';

export class ChapterView {
  constructor(scutoon) {
    this.scutoon = scutoon;
    this.audioManager = null;
    this.sliceObserver = null;
    this.hotzoneCleanup = null;
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

    // Load season data for navigation (default to season 1)
    const seasonData = await loadSeason(this.scutoon.lang, 1);
    this.scutoon.season = seasonData;

    // Load all sequence items for navigation (needed to check status)
    const sequenceItems = await this.loadSequenceItems(seasonData);

    // Render key visual
    const keyVisualHtml = renderKeyVisual(data.keyvisual, data.title);

    // Create placeholder HTML for images
    // TODO convey a concept that allows for various render types for the slices
    // As described in InteractionManager.js
    // Perhapse it makes sense to move this to a separate component file
    // and to separate concerns of rendering vs interaction handling
    let slicesHtml = '';
    if (data.slices && data.slices.length > 0) {
      slicesHtml = data.slices.map((slice, index) => renderChapterSlice(slice, index)).join('');
    }

    // Render sequence navigation
    const navigationHtml = renderSequenceNavigation({
      sequenceItems: sequenceItems,
      currentType: 'chapter',
      currentNumber: chapterNumber
    });

    const content = `
      <div class="chapter-view">
        <div class="webtoon-container">
          ${keyVisualHtml}
          ${slicesHtml}
          ${navigationHtml}
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

    // Initialize hotzone visualizer
    this.hotzoneCleanup = initHotzoneVisualizer();

    // Start sequential lazy loading
    if (data.slices && data.slices.length > 0) {
      loadImagesSequentially();
    }

    return data;
  }

  async loadSequenceItems(seasonData) {
    const sequenceItems = [];
    for (const item of seasonData.sequence) {
      try {
        let itemData;
        if (item.type === 'song') {
          const match = item.file.match(/song(\d+)\.json/);
          if (!match) continue;
          const songNumber = sanitizeNumber(match[1], 1, 1, 999);
          itemData = await loadSong(this.scutoon.lang, songNumber);
          itemData.itemType = 'song';
          itemData.itemNumber = songNumber;
        } else if (item.type === 'chapter') {
          const match = item.file.match(/chapter(\d+)\.json/);
          if (!match) continue;
          const chapterNumber = sanitizeNumber(match[1], 1, 1, 999);
          itemData = await loadChapter(this.scutoon.lang, chapterNumber);
          itemData.itemType = 'chapter';
          itemData.itemNumber = chapterNumber;
        }
        if (itemData) {
          sequenceItems.push(itemData);
        }
      } catch (error) {
        console.error(`Failed to load ${item.type} from ${item.file}:`, error);
        // Continue loading other items even if one fails
      }
    }
    return sequenceItems;
  }

  async cleanup() {
    // Start audio fade out in background (don't wait for it)
    if (this.audioManager) {
      this.audioManager.cleanup(); // Fades out in background
      this.audioManager = null;
    }

    // Cleanup slice observer
    if (this.sliceObserver) {
      this.sliceObserver.disconnect();
      this.sliceObserver = null;
    }

    // Cleanup hotzone observer
    if (this.hotzoneCleanup) {
      this.hotzoneCleanup();
      this.hotzoneCleanup = null;
    }

    console.log('ChapterView cleaned up');
  }
}
