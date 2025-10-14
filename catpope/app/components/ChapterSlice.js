/**
 * ChapterSlice component - renders a webtoon image slice placeholder
 */
import { sanitizeUrl, escapeHtml, escapeAttr } from '../utils/template.js';

export function renderChapterSlice(slice, index) {
  const imageUrl = sanitizeUrl(slice.image);
  if (!imageUrl) {
    console.warn(`Invalid image URL at slice ${index + 1}`);
    return '';
  }
  
  const sliceNumber = index + 1;
  return `
    <div class="webtoon-slice lazy" data-slice="${escapeAttr(sliceNumber)}">
      <div class="slice-placeholder">Loading image ${escapeHtml(sliceNumber)}...</div>
      <img data-src="${escapeHtml(imageUrl)}" alt="${escapeAttr('Slice ' + sliceNumber)}" style="display: none;">
    </div>
  `;
}
