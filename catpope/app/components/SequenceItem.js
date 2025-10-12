/**
 * SequenceItem component - renders mixed chapter/song items in sequence
 */
import { renderChapterItem } from './ChapterItem.js';
import { renderSongItem } from './SongItem.js';

export function renderSequenceItem(item) {
  if (item.itemType === 'song') {
    return renderSongItem(item, item.itemNumber);
  } else if (item.itemType === 'chapter') {
    return renderChapterItem(item, item.itemNumber);
  }
  return '';
}
