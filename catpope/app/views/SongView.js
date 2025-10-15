/**
 * SongView - Song player and details
 */
import { loadSong, loadSeason, loadChapter } from '../utils/api.js';
import { renderSequenceNavigation } from '../components/SequenceNavigation.js';
import { MembershipRequiredView } from './MembershipRequiredView.js';
import { escapeHtml, sanitizeUrl, escapeAttr, sanitizeNumber } from '../utils/template.js';

export class SongView {
  constructor(scutoon) {
    this.scutoon = scutoon;
  }

  async render(songNumber) {
    // Check membership before rendering song
    if (!this.scutoon.membershipStore.hasValidMembership()) {
      const membershipView = new MembershipRequiredView(this.scutoon);
      membershipView.render(null, songNumber);
      return null;
    }

    const songData = await loadSong(this.scutoon.lang, songNumber);
    this.scutoon.song = songData;

    // Load season data for navigation (default to season 1)
    const seasonData = await loadSeason(this.scutoon.lang, 1);
    this.scutoon.season = seasonData;

    // Load all sequence items for navigation (needed to check status)
    const sequenceItems = await this.loadSequenceItems(seasonData);

    let scoreHtml = '';
    if (songData.scores) {
      scoreHtml = songData.scores.map(score => {
        if (score.status === 'draft') {
          return '';
        }
        const scoreUrl = sanitizeUrl(score.src);
        if (!scoreUrl) return '';
        return `
        <div class="song-score">
          <div class="song-score-title">${escapeHtml(score.name)}</div>
          <div class="song-score-src"><a href="${escapeHtml(scoreUrl)}" target="_blank">Download</a></div>
        </div>
      `;
      }).join('');
    }

    let autoplayAudio = true;

    let videoHtml = '';
    if (songData.video && songData.video.sources && songData.video.sources.youtube && songData.video.sources.youtube.id) {
      // Validate YouTube video ID (alphanumeric, dash, underscore, 11 chars typical)
      const videoId = String(songData.video.sources.youtube.id).trim();
      if (/^[a-zA-Z0-9_-]{11}$/.test(videoId)) {
        videoHtml = `
          <div class="song-video">
            <iframe 
              width="100%" 
              height="315" 
              src="https://www.youtube.com/embed/${escapeAttr(videoId)}?autoplay=1" 
              frameborder="0" 
              allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" 
              allowfullscreen>
            </iframe>
          </div>
        `;
        autoplayAudio = false;
      }
    }

    let audioHtml = '';
    if (songData.src) {
      const audioUrl = sanitizeUrl(songData.src);
      if (audioUrl) {
        audioHtml = `
          <div class="song-src"><audio controls ${autoplayAudio ? 'autoplay' : ''}><source src="${escapeHtml(audioUrl)}" type="audio/mpeg"></audio></div>
        `;
      }
    }

    // Render sequence navigation
    const navigationHtml = renderSequenceNavigation({
      sequenceItems: sequenceItems,
      currentType: 'song',
      currentNumber: songNumber
    });

    const content = `
      <div class="view song-view">
        ${videoHtml}
        <div class="song-header">
          <div class="song-title">${escapeHtml(songData.title)}</div>
          <div class="song-description">${escapeHtml(songData.description)}</div>
          ${audioHtml}
        </div>
        ${scoreHtml}
        ${navigationHtml}
      </div>
    `;

    if (this.scutoon.contentEl) {
      this.scutoon.contentEl.innerHTML = content;
      this.scutoon.contentEl.classList.remove('chapter-mode');
    }

    return songData;
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
}
