/**
 * NavigationStore - Handles routing and navigation state
 */
export class NavigationStore {
  currentView = 'toon'; // toon, season, chapter, song, account, credits, player
  currentChapter = 1;
  currentSeason = 1;
  currentSong = 1;

  constructor() {
    // Initialize from URL if present
    this.initFromURL();
  }

  initFromURL() {
    const urlParams = new URLSearchParams(window.location.search);
    this.currentView = urlParams.get('view') || 'toon';
    this.currentChapter = parseInt(urlParams.get('chapter')) || 1;
    this.currentSeason = parseInt(urlParams.get('season')) || 1;
    this.currentSong = parseInt(urlParams.get('song')) || 1;
  }

  // Simple router - update URL without page reload
  updateURL(params) {
    const currentParams = new URLSearchParams(window.location.search);

    // Update or set new parameters
    Object.keys(params).forEach(key => {
      if (params[key] !== undefined && params[key] !== null) {
        currentParams.set(key, params[key]);
      }
    });

    // Update URL without reload
    const newURL = window.location.pathname + '?' + currentParams.toString();
    window.history.pushState({ ...params }, '', newURL);
  }

  // Update current view state
  setView(view, params = {}) {
    this.currentView = view;
    if (params.chapter) this.currentChapter = params.chapter;
    if (params.season) this.currentSeason = params.season;
    if (params.song) this.currentSong = params.song;
  }
}
