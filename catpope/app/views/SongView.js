/**
 * @typedef {import('./types').ISongView} ISongView
 */

import { loadSong, loadSeason } from '../utils/api.js';
import { loadSequenceItems } from '../utils/sequence.js';
import { renderSequenceNavigation } from '../components/SequenceNavigation.js';
import { MembershipRequiredView } from './MembershipRequiredView.js';
import { escapeHtml, escapeAttr } from '../utils/template.js';
import { asset } from '../utils/cdn.js';

/** @implements {ISongView} */
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

    const songData = await loadSong(this.scutoon.navigationStore.currentLang, songNumber);

    // Load season data for navigation (default to season 1)
    const seasonData = await loadSeason(this.scutoon.navigationStore.currentLang, 1);

    // Load all sequence items for navigation (needed to check status)
    const sequenceItems = await loadSequenceItems(seasonData, this.scutoon.navigationStore.currentLang);

    let scoreHtml = '';
    if (songData.scores) {
      scoreHtml = songData.scores.map(score => {
        if (score.status === 'draft') {
          return '';
        }
        if (!scoreUrl) return '';
        return `
        <div class="song-score">
          <div class="song-score-title">${escapeHtml(score.name)}</div>
          <div class="song-score-src"><a href="${asset(score.src)}" target="_blank">Download</a></div>
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
      const audioUrl = asset(songData.src);
      if (audioUrl) {
        audioHtml = `
          <div class="song-src"><audio controls ${autoplayAudio ? 'autoplay' : ''}><source src="${audioUrl}" type="audio/mpeg"></audio></div>
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
}
