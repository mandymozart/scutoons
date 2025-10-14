/**
 * ChapterItem component - renders a single chapter card
 */
import { escapeHtml, sanitizeUrl, escapeAttr, sanitizeNumber } from '../utils/template.js';

export function renderChapterItem(chapter, chapterNumber) {
  const safeChapterNumber = sanitizeNumber(chapterNumber, 1, 1, 100);
  const thumbnailUrl = sanitizeUrl(chapter.thumbnail || 'assets/images/default-thumbnail.jpg');
  
  if (chapter.status === 'published') {
    return `
      <div class="chapter-item published" onclick="scutoon.navigateTo('chapter', {chapter: ${safeChapterNumber}})">
        <div class="chapter-image"><img class="chapter-thumbnail" src="${escapeHtml(thumbnailUrl)}" alt="${escapeAttr('Kapitel ' + chapter.number + ' Thumbnail')}" loading="lazy"></div>
        <div class="chapter-title">${escapeHtml(chapter.title)}</div>
        <div class="chapter-release">${escapeHtml(chapter.release)}</div>
        <div class="chapter-number"><img class="chapter-icon" src="../assets/icons/chapter.svg" alt="Chapter"> #${escapeHtml(chapter.number)}</div>
      </div>
    `;
  } else if (chapter.status === 'coming-soon') {
    return `
      <div class="chapter-item coming-soon">
        <div class="chapter-image"><img class="chapter-thumbnail" src="${escapeHtml(thumbnailUrl)}" alt="${escapeAttr('Kapitel ' + chapter.number + ' Thumbnail')}" loading="lazy"></div>
        <div class="chapter-title">${escapeHtml(chapter.title)}</div>
        <div class="chapter-release"><span class="badge">Coming Soon</span><br/>${escapeHtml(chapter.release)}</div>
        <div class="chapter-number"><img class="chapter-icon" src="../assets/icons/chapter.svg" alt="Chapter">#${escapeHtml(chapter.number)}</div>
      </div>
    `;
  }
  return '';
}
