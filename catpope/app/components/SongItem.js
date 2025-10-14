/**
 * SongItem component - renders a single song card
 */
import { escapeHtml, sanitizeUrl, escapeAttr, sanitizeNumber } from '../utils/template.js';

export function renderSongItem(song, songNumber) {
  const safeSongNumber = sanitizeNumber(songNumber, 1, 1, 100);
  const thumbnailUrl = sanitizeUrl(song.thumbnail || 'assets/images/default-thumbnail.jpg');
  
  if (song.status === 'published') {
    return `
      <div class="chapter-item song published" onclick="scutoon.navigateTo('song', {song: ${safeSongNumber}})">
        <div class="chapter-image"><img class="chapter-thumbnail" src="${escapeHtml(thumbnailUrl)}" alt="${escapeAttr(song.title + ' Thumbnail')}" loading="lazy"></div>
        <div class="chapter-title">${escapeHtml(song.title)}</div>
        <div class="chapter-release">${escapeHtml(song.release)}</div>
        <div class="chapter-number"><img class="chapter-icon" src="../assets/icons/headphones.svg" alt="Song"> #${escapeHtml(song.number)}</div>
      </div>
    `;
  } else if (song.status === 'coming-soon') {
    return `
      <div class="chapter-item song coming-soon">
        <div class="chapter-image"><img class="chapter-thumbnail" src="${escapeHtml(thumbnailUrl)}" alt="${escapeAttr(song.title + ' Thumbnail')}" loading="lazy"></div>
        <div class="chapter-title">${escapeHtml(song.title)}</div>
        <div class="chapter-release"><span class="badge">Coming Soon</span><br/>${escapeHtml(song.release)}</div>
        <div class="chapter-number"><img class="chapter-icon" src="../assets/icons/headphones.svg" alt="Song"> #${escapeHtml(song.number)}</div>
      </div>
    `;
  }
  return '';
}
