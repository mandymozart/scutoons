/**
 * @typedef {import('../types').LoadedSequenceItem} LoadedSequenceItem
 * @typedef {import('../types').Season} Season
 */

import { loadChapter, loadSong } from './api.js';
import { sanitizeNumber } from './template.js';

/**
 * @param {Season} seasonData
 * @param {string} lang
 * @returns {Promise<LoadedSequenceItem[]>}
 */
export async function loadSequenceItems(seasonData, lang) {
  /** @type {LoadedSequenceItem[]} */
  const sequenceItems = [];
  
  for (const item of seasonData.sequence) {
    try {
      if (item.type === 'song') {
        const match = item.file.match(/song(\d+)\.json/);
        if (!match) continue;
        const songNumber = sanitizeNumber(match[1], 1, 1, 999);
        const songData = await loadSong(lang, songNumber);
        sequenceItems.push(/** @type {LoadedSequenceItem} */ ({
          ...songData,
          itemType: 'song',
          itemNumber: songNumber
        }));
      } else if (item.type === 'chapter') {
        const match = item.file.match(/chapter(\d+)\.json/);
        if (!match) continue;
        const chapterNumber = sanitizeNumber(match[1], 1, 1, 999);
        const chapterData = await loadChapter(lang, chapterNumber);
        sequenceItems.push(/** @type {LoadedSequenceItem} */ ({
          ...chapterData,
          itemType: 'chapter',
          itemNumber: chapterNumber
        }));
      }
    } catch (error) {
      console.error(`Failed to load ${item.type} from ${item.file}:`, error);
    }
  }
  
  return sequenceItems;
}

/**
 * @param {LoadedSequenceItem[]} sequenceItems
 * @param {string} itemType
 * @param {number} itemNumber
 * @returns {number}
 */
export function findCurrentPosition(sequenceItems, itemType, itemNumber) {
  return sequenceItems.findIndex(item => 
    item.itemType === itemType && item.itemNumber === itemNumber
  );
}

/**
 * @param {LoadedSequenceItem[]} sequenceItems
 * @param {number} currentPosition
 * @returns {LoadedSequenceItem | null}
 */
export function getNextItem(sequenceItems, currentPosition) {
  if (currentPosition < sequenceItems.length - 1) {
    return sequenceItems[currentPosition + 1];
  }
  return null;
}

/**
 * @param {LoadedSequenceItem[]} sequenceItems
 * @param {number} currentPosition
 * @returns {LoadedSequenceItem | null}
 */
export function getPreviousItem(sequenceItems, currentPosition) {
  if (currentPosition > 0) {
    return sequenceItems[currentPosition - 1];
  }
  return null;
}
