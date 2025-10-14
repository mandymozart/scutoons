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
    
    // Initialize stores
    this.membershipStore = new MembershipStore();
    this.navigationStore = new NavigationStore();
    
    // Initialize scroll handler
    initScrollHandler();
  }

  // Navigate to a specific view through the router
  navigateTo(view, params = {}) {
    const navigationParams = { view, ...params };

    // Update URL first
    this.navigationStore.updateURL(navigationParams);

    // Then render the appropriate view
    switch (view) {
      case 'chapter':
        this.renderChapterView(sanitizeNumber(params.chapter, 1, 1, 100));
        break;
      case 'song':
        this.renderSongView(sanitizeNumber(params.song, 1, 1, 100));
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

    // Scroll to top after view transition
    scrollToTop();
  }

  // View rendering methods
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

  // Check for new QR code (manual trigger)
  async checkForNewCode() {
    const urlParams = new URLSearchParams(window.location.search);
    const code = urlParams.get('code');

    if (code) {
      const result = await this.membershipStore.validateQRCode(code);
      if (result && result.success) {
        // If validation successful, try to render the chapter again
        const chapter = sanitizeNumber(urlParams.get('chapter'), 1, 1, 100);
        this.renderChapterView(chapter);
      } else {
        alert('Kein gültiger Mitgliedschaftscode gefunden.');
      }
    } else {
      alert('Kein QR-Code in der URL gefunden. Bitte scannen Sie einen gültigen SCUTOONS QR-Code.');
    }
  }

  // Initialize the app with a specific view
  async init(initialView = 'toon', initialChapter = 1, initialSeason = 1, initialSong = 1) {
    // First process any QR code from URL
    const qrResult = await this.membershipStore.processQRCodeFromURL();

    if (qrResult && qrResult.success) {
      console.log('Successfully validated membership from QR code');
    }

    renderAccountNavigation(this.membershipStore.membership);

    switch (initialView) {
      case 'chapter':
        await this.renderChapterView(sanitizeNumber(initialChapter, 1, 1, 100));
        break;
      case 'song':
        await this.renderSongView(sanitizeNumber(initialSong, 1, 1, 100));
        break;
      case 'account':
        await this.renderAccountView();
        break;
      case 'credits':
        await this.renderCreditsView();
        break;
      default:
        await this.renderToonView();
    }

    // Scroll to top after view transition
    window.scrollTo(0, 0);
  }
}
