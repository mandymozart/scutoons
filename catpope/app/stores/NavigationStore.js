/**
 * @typedef {import('../types').Language} Language
 * @typedef {import('../types').View} View
 * @typedef {import('./types').INavigationStore} INavigationStore
 */

import { Language, View } from '../constants.js';

/**
 * @implements {INavigationStore}
 */
export class NavigationStore {
  /** @type {View} */
  currentView = View.TOON;
  /** @type {number} */
  currentChapter = 1;
  /** @type {number} */
  currentSong = 1;
  /** @type {string | null} */
  currentLabel = null;
  /** @type {Language} */
  currentLang = Language.DE;
  /** @type {import('../types').IViewInstance | null} */
  currentViewInstance = null;

  setView(view, params = {}) {
    this.currentView = view;
    if (params.chapter !== undefined) this.currentChapter = params.chapter;
    if (params.song !== undefined) this.currentSong = params.song;
    if (params.label !== undefined) this.currentLabel = params.label;
    if (params.lang !== undefined) this.currentLang = params.lang;
  }

  setViewInstance(viewInstance) {
    this.currentViewInstance = viewInstance;
  }

  async cleanupCurrentView() {
    if (this.currentViewInstance && typeof this.currentViewInstance.cleanup === 'function') {
      await this.currentViewInstance.cleanup();
    }
    this.currentViewInstance = null;
  }

  getState() {
    return {
      view: this.currentView,
      chapter: this.currentChapter,
      song: this.currentSong,
      label: this.currentLabel,
      lang: this.currentLang
    };
  }
}
