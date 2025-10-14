/**
 * SCUTOONS Base App, used by all SCUTOONS
 */
import { TOON_ID } from './config.js';
import { MembershipStore, NavigationStore } from './stores/index.js';
import { initScrollHandler, scrollToTop } from './utils/index.js';
import { renderAccountNavigation } from './components/Navigation.js';
import { sanitizeNumber } from './utils/template.js';
import {
  ToonView,
  ChapterView,
  SongView,
  AccountView,
  CreditsView,
} from './views/index.js';

export class Scutoon {
  lang;
  menuEl;
  contentEl;
  membershipStore;
  navigationStore;

  constructor(lang) {
    console.log(`Scutoon "${TOON_ID}" initialized for language "${lang}"`);
    this.lang = lang;
    this.menuEl = document.querySelector('.nav .menu');
    this.contentEl = document.querySelector('.content');
    
    this.membershipStore = new MembershipStore();
    this.navigationStore = new NavigationStore();
    initScrollHandler();
  }

  navigateTo(view, params = {}) {
    const navigationParams = { view, ...params };
    this.navigationStore.updateURL(navigationParams);
    switch (view) {
      case 'chapter':
        this.renderChapterView(sanitizeNumber(params.chapter, 1, 1, 999));
        break;
      case 'song':
        this.renderSongView(sanitizeNumber(params.song, 1, 1, 999));
        break;
      case 'account':
        this.renderAccountView();
        break;
      case 'credits':
        this.renderCreditsView();
        break;
      default:
        this.renderToonView();
    }
    scrollToTop();
  }

  async renderToonView() {
    const view = new ToonView(this);
    return await view.render();
  }

  async renderChapterView(chapterNumber = 1) {
    const view = new ChapterView(this);
    return await view.render(chapterNumber);
  }

  async renderSongView(songNumber) {
    const view = new SongView(this);
    return await view.render(songNumber);
  }


  renderAccountView() {
    const view = new AccountView(this);
    view.render();
  }

  async renderCreditsView() {
    const view = new CreditsView(this);
    await view.render();
  }

  async init(initialView = 'toon', initialChapter = 1, initialSong = 1) {
    const qrResult = await this.membershipStore.processQRCodeFromURL();

    if (qrResult && qrResult.success) {
      console.log('Successfully validated membership from QR code');
    }

    renderAccountNavigation(this.membershipStore.membership);

    switch (initialView) {
      case 'chapter':
        await this.renderChapterView(sanitizeNumber(initialChapter, 1, 1, 999));
        break;
      case 'song':
        await this.renderSongView(sanitizeNumber(initialSong, 1, 1, 999));
        break;
      case 'account':
        this.renderAccountView();
        break;
      case 'credits':
        await this.renderCreditsView();
        break;
      default:
        await this.renderToonView();
    }

    scrollToTop();
  }
}
