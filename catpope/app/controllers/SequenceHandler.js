/**
 * @typedef {import('../types').LoadedSequenceItem} LoadedSequenceItem
 * @typedef {import('../types').Season} Season
 */

import { loadSeason } from '../utils/api.js';
import { loadSequenceItems } from '../utils/sequence.js';

/**
 * SequenceHandler - Manages season sequence data and navigation
 */
export class SequenceHandler {
  /**
   * @param {string} lang
   * @param {number} seasonNumber
   */
  constructor(lang, seasonNumber = 1) {
    this.lang = lang;
    this.seasonNumber = seasonNumber;
    
    /** @type {Season | null} */
    this.seasonData = null;
    
    /** @type {LoadedSequenceItem[]} */
    this.sequenceItems = [];
    
    this.loaded = false;
  }

  /**
   * @returns {Promise<void>}
   */
  async load() {
    if (this.loaded) return;
    
    this.seasonData = await loadSeason(this.lang, this.seasonNumber);
    this.sequenceItems = await loadSequenceItems(this.seasonData, this.lang);
    this.loaded = true;
    
    console.log(`Loaded ${this.sequenceItems.length} sequence items for season ${this.seasonNumber}`);
  }

  /**
   * @returns {LoadedSequenceItem[]}
   */
  getSequenceItems() {
    return this.sequenceItems;
  }

  /**
   * @returns {Season | null}
   */
  getSeasonData() {
    return this.seasonData;
  }
}
