/**
 * ToonView - Main overview with chapters/songs list
 */
import { loadToon, loadSeason, loadChapter, loadSong } from '../utils/api.js';
import { renderSequenceItem } from '../components/SequenceItem.js';
import { escapeHtml, sanitizeNumber } from '../utils/template.js';
import { asset } from '../utils/cdn.js';

export class ToonView {
  constructor(scutoon) {
    this.scutoon = scutoon;
  }

  async render() {
    const data = await loadToon(this.scutoon.lang);
    this.scutoon.toon = data;

    // Display download options
    let downloadHtml = ``;
    if (data.download && data.download.music && this.scutoon.membershipStore.hasValidMembership()) {
      const musicUrl = asset(data.download.music);
      if (musicUrl) {
        downloadHtml = `<div class="toon-download"><a class="btn" href="${musicUrl}" target="_blank">Download Music</a></div>`;
      }
    }

    // Load season1 data to get the sequence
    const seasonData = await loadSeason(this.scutoon.lang, 1);
    this.scutoon.season = seasonData;

    // Load all items from the season sequence
    const sequenceItems = [];
    for (const item of seasonData.sequence) {
      try {
        let itemData;
        if (item.type === 'song') {
          // Extract song number from filename (e.g., "song1.json" -> 1)
          const match = item.file.match(/song(\d+)\.json/);
          if (!match) {
            throw new Error(`Invalid song file format: ${item.file}`);
          }
          const songNumber = sanitizeNumber(match[1], 1, 1, 999);
          itemData = await loadSong(this.scutoon.lang, songNumber);
          itemData.itemType = 'song';
          itemData.itemNumber = songNumber;
        } else if (item.type === 'chapter') {
          // Extract chapter number from filename (e.g., "chapter1.json" -> 1)
          const match = item.file.match(/chapter(\d+)\.json/);
          if (!match) {
            throw new Error(`Invalid chapter file format: ${item.file}`);
          }
          const chapterNumber = sanitizeNumber(match[1], 1, 1, 999);
          itemData = await loadChapter(this.scutoon.lang, chapterNumber);
          itemData.itemType = 'chapter';
          itemData.itemNumber = chapterNumber;
        }
        sequenceItems.push(itemData);
      } catch (error) {
        console.error(`Failed to load ${item.type} from ${item.file}:`, error);
        throw error;
      }
    }

    // Render sequence items (mix of songs and chapters)
    const sequenceHtml = sequenceItems.map(item => renderSequenceItem(item)).join('');

    const content = `
      <div class="toon-overview">
        <div class="toon-header">
          <img src="${asset(data.cover.square)}" alt="Cover" class="Cover" />
          <h1>${escapeHtml(data.title)}</h1>
          <p class="description">${escapeHtml(data.description)}</p>
          ${downloadHtml}
        </div>
        
        <section class="chapters-section">
        <div class="chapters-header">
          <h2>Season 1: ${escapeHtml(seasonData.title)}</h2>
          <p class="season-description">${escapeHtml(seasonData.description)}</p>
        </div>
          <div class="chapters-list">
            ${sequenceHtml}
          </div>
        </section>
        <section class="credits-section">
         <button onclick="scutoon.navigateTo('credits')">Credits</button>
        </section>
      </div>
    `;

    if (this.scutoon.contentEl) {
      this.scutoon.contentEl.innerHTML = content;
      this.scutoon.contentEl.classList.remove('chapter-mode');
    }

    return data;
  }
}
