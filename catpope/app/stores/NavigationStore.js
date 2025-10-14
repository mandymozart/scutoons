export class NavigationStore {
  currentView = 'toon';
  currentChapter = 1;
  currentSong = 1;

  setView(view, params = {}) {
    this.currentView = view;
    if (params.chapter) this.currentChapter = params.chapter;
    if (params.song) this.currentSong = params.song;
  }

  getState() {
    return {
      view: this.currentView,
      chapter: this.currentChapter,
      song: this.currentSong
    };
  }
}
