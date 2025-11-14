// Main application interface
import { IGameStore, IMembershipStore, INavigationStore, IVisualNovelStore } from './stores/types';

export interface IScutoon {
  menuEl: Element | null;
  contentEl: Element | null;
  membershipStore: IMembershipStore;
  navigationStore: INavigationStore;
  gameStore: IGameStore;
  visualNovelStore: IVisualNovelStore;
  navigateTo(view: View, params?: any): void;
  renderToonView(): Promise<any>;
  renderChapterView(chapterNumber?: number): Promise<any>;
  renderSongView(songNumber: number): Promise<any>;
  renderAccountView(): Promise<void>;
  renderCreditsView(): Promise<void>;
  init(): Promise<void>;
}

// Re-exports for convenience
export * from './models/enums';
export * from './models/audio';
export * from './models/visual-novel';
export * from './models/content';
export * from './models/membership';
export * from './stores/types';
export * from './controllers/events/types';
export * from './controllers/interactions/types';
export * from './renderers/types';
export * from './views/types';
