import { TOON_ID } from './config.js';
import { MembershipStore, NavigationStore } from './stores/index.js';
import { initScrollHandler, navigateToView } from './utils/index.js';
import { renderAccountNavigation } from './components/Navigation.js';
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
    return navigateToView(this, view, params);
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

  async init() {
    const qrResult = await this.membershipStore.processQRCodeFromURL();
    if (qrResult && qrResult.success) {
      console.log('Successfully validated membership from QR code');
    }
    renderAccountNavigation(this.membershipStore.membership);
  }
}
