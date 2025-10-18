/**
 * ChapterSlice component - renders a webtoon image slice placeholder
 */
import { escapeHtml, escapeAttr } from '../utils/template.js';
import { asset } from '../utils/cdn.js';

export function renderChapterSlice(slice, index) {
  const imageUrl = slice.image ? asset(slice.image) : null;
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
      <div class="slice-image-wrapper">
        <img src="../assets/1x1-pixel.png" class="slice-placeholder-img" alt="">
        <img data-src="${imageUrl}" class="slice-content-img" alt="${escapeAttr('Slice ' + sliceNumber)}" style="display: none;">
        <div class="slice-loading-text">Loading slice ${escapeHtml(sliceNumber)}...</div>
      </div>
      ${hotzonesHtml}
    </div>
  `;
}
