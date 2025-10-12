/**
 * ChapterSlice component - renders a webtoon image slice placeholder
 */
export function renderChapterSlice(slice, index) {
  return `
    <div class="webtoon-slice lazy" data-slice="${index + 1}">
      <div class="slice-placeholder">Loading image ${index + 1}...</div>
      <img data-src="${slice.image}" alt="Slice ${index + 1}" style="display: none;">
    </div>
  `;
}
