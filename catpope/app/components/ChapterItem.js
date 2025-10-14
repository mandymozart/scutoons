/**
 * ChapterItem component - renders a single chapter card
 */
export function renderChapterItem(chapter, chapterNumber) {
  if (chapter.status === 'published') {
    return `
      <div class="chapter-item published" onclick="scutoon.navigateTo('chapter', {chapter: ${chapterNumber}})">
        <div class="chapter-image"><img class="chapter-thumbnail" src="${chapter.thumbnail || 'assets/images/default-thumbnail.jpg'}" alt="Kapitel ${chapter.number} Thumbnail" loading="lazy"></div>
        <div class="chapter-title">${chapter.title}</div>
        <div class="chapter-release">${chapter.release}</div>
        <div class="chapter-number"><img class="chapter-icon" src="../assets/icons/chapter.svg" alt="Chapter"> #${chapter.number}</div>
      </div>
    `;
  } else if (chapter.status === 'coming-soon') {
    return `
      <div class="chapter-item coming-soon">
        <div class="chapter-image"><img class="chapter-thumbnail" src="${chapter.thumbnail || 'assets/images/default-thumbnail.jpg'}" alt="Kapitel ${chapter.number} Thumbnail" loading="lazy"></div>
        <div class="chapter-title">${chapter.title}</div>
        <div class="chapter-release"><span class="badge">Coming Soon</span><br/>${chapter.release}</div>
        <div class="chapter-number"><img class="chapter-icon" src="../assets/icons/chapter.svg" alt="Chapter">#${chapter.number}</div>
      </div>
    `;
  }
  return '';
}
