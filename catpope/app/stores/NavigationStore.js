export class NavigationStore {
  currentView = 'toon';
  currentChapter = 1;
  currentSong = 1;
  currentViewInstance = null;

  setView(view, params = {}) {
    this.currentView = view;
    if (params.chapter) this.currentChapter = params.chapter;
    if (params.song) this.currentSong = params.song;
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
      song: this.currentSong
    };
  }
}
