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
  
  // Check for interactive hotzones (click/hover triggers with rects)
  let hotzonesHtml = '';
  if (slice.audio?.samples) {
    slice.audio.samples.forEach((sample, sampleIndex) => {
      const trigger = sample.trigger;
      if (trigger?.rect && (trigger.type === 'click' || trigger.type === 'hover')) {
        const rect = trigger.rect;
        hotzonesHtml += `
          <div class="audio-hotzone" 
               style="left: ${rect.x}%; top: ${rect.y}%; width: ${rect.width}%; height: ${rect.height}%;"
               data-trigger-type="${trigger.type}">
          </div>
        `;
      }
    });
  }
  
  return `
    <div class="webtoon-slice lazy" data-slice="${escapeAttr(sliceNumber)}">
      <div class="slice-placeholder">Loading image ${escapeHtml(sliceNumber)}...</div>
      <img data-src="${escapeHtml(imageUrl)}" alt="${escapeAttr('Slice ' + sliceNumber)}" style="display: none;">
      ${hotzonesHtml}
    </div>
  `;
}
