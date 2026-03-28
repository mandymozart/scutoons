import { TOON_ID } from './config.js';
import { MembershipStore, NavigationStore, GameStore, VisualNovelStore } from './stores/index.js';
import { navigateToView } from './utils/index.js';
import { renderAccountNavigation } from './components/AccountNavigation.js';
import { renderHeader, initHeader } from './components/Header.js';
import {
  ToonView,
  ChapterView,
  SongView,
  AccountView,
  CreditsView,
} from './views/index.js';

/**
 * @typedef {import('./types').IScutoon} IScutoon
 */

/**
 * @implements {IScutoon}
 */
export class Scutoon {
  menuEl;
  contentEl;
  membershipStore;
  navigationStore;
  gameStore;
  visualNovelStore;

  constructor() {
    console.log(`Scutoon "${TOON_ID}" initialized`);
    this.menuEl = document.querySelector('.nav .menu');
    this.contentEl = document.querySelector('.content');
    
    this.membershipStore = new MembershipStore();
    this.navigationStore = new NavigationStore();
    this.gameStore = new GameStore(TOON_ID);
    this.visualNovelStore = new VisualNovelStore(TOON_ID, this.navigationStore.currentLang);
  }

  /**
   * @param {import('./types').View} view
   * @param {Object} [params={}]
   */
  navigateTo(view, params = {}) {
    return navigateToView(this, view, params);
  }

  /**
   * @returns {Promise<any>} Rendered view data
   */
  async renderToonView() {
    await this.navigationStore.cleanupCurrentView();
    const view = new ToonView(this);
    this.navigationStore.setViewInstance(view);
    return await view.render();
  }

  /**
   * @param {number} [chapterNumber=1] - Chapter number to render
   * @returns {Promise<any>} Chapter data
   */
  async renderChapterView(chapterNumber = 1) {
    await this.navigationStore.cleanupCurrentView();
    const view = new ChapterView(this);
    this.navigationStore.setViewInstance(view);
    return await view.render(chapterNumber);
  }

  /**
   * @param {number} songNumber - Song number to render
   * @returns {Promise<any>} Song data
   */
  async renderSongView(songNumber) {
    await this.navigationStore.cleanupCurrentView();
    const view = new SongView(this);
    this.navigationStore.setViewInstance(view);
    return await view.render(songNumber);
  }

  /**
   * @returns {Promise<void>}
   */
  async renderAccountView() {
    await this.navigationStore.cleanupCurrentView();
    const view = new AccountView(this);
    this.navigationStore.setViewInstance(view);
    view.render();
  }

  /**
   * @returns {Promise<void>}
   */
  async renderCreditsView() {
    await this.navigationStore.cleanupCurrentView();
    const view = new CreditsView(this);
    this.navigationStore.setViewInstance(view);
    await view.render();
  }

  /**
   * @returns {Promise<void>}
   */
  async init() {
    // Render header
    const headerHtml = await renderHeader(this.navigationStore.currentLang);
    document.body.insertAdjacentHTML('afterbegin', headerHtml);
    initHeader();
    
    // Re-select menu after header is rendered
    this.menuEl = document.querySelector('.nav .menu');
    
    // Process membership and render account navigation
    const qrResult = await this.membershipStore.processQRCodeFromURL();
    if (qrResult && qrResult.success) {
      console.log('Successfully validated membership from QR code');
    }
    renderAccountNavigation(this.membershipStore.membership);
  }
}
