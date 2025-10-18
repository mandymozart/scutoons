/**
 * SongItem component - renders a single song card
 */
import { escapeHtml, escapeAttr, sanitizeNumber } from '../utils/template.js';
import { asset } from '../utils/cdn.js';

export function renderSongItem(song, songNumber) {
  const safeSongNumber = sanitizeNumber(songNumber, 1, 1, 100);
  const thumbnailUrl = asset(song.thumbnail || 'images/default-thumbnail.jpg');
  const isPublished = song.status === 'published';
  const status = song.status || 'published';
  
  if (!['published', 'coming-soon'].includes(status)) return '';
  
  return `
    <div class="chapter-item song ${status}"${isPublished ? ` onclick="scutoon.navigateTo('song', {song: ${safeSongNumber}})"` : ''}>
      <div class="chapter-image"><img class="chapter-thumbnail" src="${thumbnailUrl}" alt="${escapeAttr(song.title + ' Thumbnail')}" loading="lazy"></div>
      <div class="chapter-info">
        <div class="chapter-title">${escapeHtml(song.title)}</div>
        <div class="chapter-release">${!isPublished ? '<span class="chapter-release-badge">Coming Soon</span>' : ''}<span class="chapter-release-date">${escapeHtml(song.release)}</span></div>
      </div>
      <div class="chapter-number"><img class="chapter-icon" src="../assets/icons/headphones.svg" alt="Song">${isPublished ? ' ' : ''}#${escapeHtml(song.number)}</div>
    </div>
  `;
}
