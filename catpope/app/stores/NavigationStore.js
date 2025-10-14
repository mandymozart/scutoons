/**
 * NavigationStore - Handles routing and navigation state
 */
export class NavigationStore {
  currentView = 'toon'; // toon, chapter, song, account, credits
  currentChapter = 1;
  currentSong = 1;

  constructor() {
    this.initFromURL();
  }

  initFromURL() {
    const urlParams = new URLSearchParams(window.location.search);
    this.currentView = urlParams.get('view') || 'toon';
    this.currentChapter = parseInt(urlParams.get('chapter')) || 1;
    this.currentSong = parseInt(urlParams.get('song')) || 1;
  }

  updateURL(params) {
    const currentParams = new URLSearchParams(window.location.search);
    Object.keys(params).forEach(key => {
      if (params[key] !== undefined && params[key] !== null) {
        currentParams.set(key, params[key]);
      }
    });
    const newURL = window.location.pathname + '?' + currentParams.toString();
    window.history.pushState({ ...params }, '', newURL);
  }

  setView(view, params = {}) {
    this.currentView = view;
    if (params.chapter) this.currentChapter = params.chapter;
    if (params.song) this.currentSong = params.song;
  }
}
