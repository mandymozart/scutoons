/**
 * ChapterItem component - renders a single chapter card
 */
import { escapeHtml, escapeAttr, sanitizeNumber, isItemPublic } from '../utils/template.js';
import { asset } from '../utils/cdn.js';

export function renderChapterItem(chapter, chapterNumber) {
  const safeChapterNumber = sanitizeNumber(chapterNumber, 1, 1, 100);
  const thumbnailUrl = asset(
    chapter.thumbnail || "images/default-thumbnail.jpg"
  );
  const isPublic = isItemPublic(chapter);
    
  const status = chapter.status || "published";

  // TODO remove, not concrete enough
  if (!["published", "coming-soon", "draft"].includes(status)) return "";

  return `
    <div class="chapter-item ${status}"${
    isPublic
      ? ` onclick="scutoon.navigateTo('chapter', {chapter: ${safeChapterNumber}})"`
      : ""
  }>
      <div class="chapter-image"><img class="chapter-thumbnail" src="${thumbnailUrl}" alt="${escapeAttr(
    "Kapitel " + chapter.number + " Thumbnail"
  )}" loading="lazy"></div>
      <div class="chapter-info">
        <div class="chapter-title">${escapeHtml(chapter.title)}</div>
        <div class="chapter-release">${
          !isPublic
            ? '<span class="chapter-release-badge">Coming Soon</span>'
            : ""
        }<span class="chapter-release-date">${escapeHtml(
    chapter.release
  )}</span></div>
      </div>
      <div class="chapter-number"><img class="chapter-icon" src="../assets/icons/chapter.svg" alt="Chapter">${
        isPublic ? " " : ""
      }#${escapeHtml(chapter.number)}</div>
    </div>
  `;
}
