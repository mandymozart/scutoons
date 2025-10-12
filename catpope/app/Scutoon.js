/**
 * SCUTOONS Base App, used by all SCUTOONS
 */
import { MembershipStore, NavigationStore } from './stores/index.js';
import { initScrollHandler, scrollToTop } from './utils/index.js';
import { renderAccountNavigation } from './components/Navigation.js';
import {
  ToonView,
  ChapterView,
  SongView,
  SeasonView,
  AccountView,
  CreditsView,
  PlayerView
} from './views/index.js';

export class Scutoon {
  toonName;
  lang;
  toon;
  chapter;
  season;
  song;
  menuEl;
  contentEl;
  membershipStore;
  navigationStore;

  constructor(toonName, lang) {
    this.toonName = toonName;
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
      case 'season':
        this.renderSeasonView(params.season || 1);
        break;
      case 'chapter':
        this.renderChapterView(params.chapter || 1);
        break;
      case 'song':
        this.renderSongView(parseInt(params.song) || 1);
        break;
      case 'account':
        this.renderAccountView();
        break;
      case 'credits':
        this.renderCreditsView();
        break;
      case 'player':
        this.renderPlayerView();
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

  async renderSeasonView(seasonNumber = 1) {
    const view = new SeasonView(this);
    return await view.render(seasonNumber);
  }

  renderAccountView() {
    const view = new AccountView(this);
    view.render();
  }

  async renderCreditsView() {
    const view = new CreditsView(this);
    await view.render();
  }

  async renderPlayerView() {
    const view = new PlayerView(this);
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
        const chapter = parseInt(urlParams.get('chapter')) || 1;
        this.renderChapterView(chapter);
      } else {
        alert('Kein gültiger Mitgliedschaftscode gefunden.');
      }
    } else {
      alert('Kein QR-Code in der URL gefunden. Bitte scannen Sie einen gültigen SCUTOONS QR-Code.');
    }
  }

  // Debug function to test membership validation (exposed for console use)
  debugMembership() {
    return this.membershipStore.debugMembership();
  }

  // Manual test to bypass attempt (for security testing)
  testBypassAttempt(fakeData) {
    return this.membershipStore.testBypassAttempt(fakeData);
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
      case 'season':
        await this.renderSeasonView(initialSeason);
        break;
      case 'chapter':
        await this.renderChapterView(initialChapter);
        break;
      case 'song':
        await this.renderSongView(initialSong);
        break;
      case 'account':
        await this.renderAccountView();
        break;
      case 'credits':
        await this.renderCreditsView();
        break;
      case 'player':
        await this.renderPlayerView();
        break;
      default:
        await this.renderToonView();
    }

    // Scroll to top after view transition
    window.scrollTo(0, 0);
  }
}
