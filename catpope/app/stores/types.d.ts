import { Language, View, IViewInstance } from '@/types';
import { Membership } from '../models/membership';

export interface GameState {
  toonId: string | null;
  progress: Record<string, any>;
  lastCheckpoint: string | null;
  variables: Record<string, string | number>;
  updated_at: string | null;
  created_at: string | null;
  platform: "web" | "app";
}

export interface IGameStore {
  qrCode: string | null;
  state: GameState;
  loadState(): void;
  saveState(): void;
  detectPlatform(): "web" | "app";
  get(key: string): string | number | undefined;
  set(key: string, value: string | number): void;
  getVariable(name: string): string | number | undefined;
  setVariable(name: string, value: string | number): void;
}

export interface IMembershipStore {
  membership: Membership | null;
  hasValidMembership(): boolean;
  processQRCodeFromURL(): Promise<any>;
  storeMembership(membershipData: Membership): boolean;
  clearMembership(): void;
}

export interface INavigationStore {
  currentView: View;
  currentChapter: number;
  currentSong: number;
  currentLabel: string | null;
  currentLang: Language;
  currentViewInstance: IViewInstance | null;
  setView(view: View, params?: any): void;
  setViewInstance(viewInstance: IViewInstance): void;
  cleanupCurrentView(): Promise<void>;
  getState(): any;
}

export interface IVisualNovelStore {
  characters: Map<string, any>;
  toonId: string;
  currentLang: string;
  pendingChoices: Array<{ button: HTMLElement, handler: () => void }>;
  loadCharacters(lang: string): Promise<void>;
  getCharacter(name: string): any;
  addChoice(choice: { button: HTMLElement, handler: () => void }): void;
  clearChoices(): void;
  clear(): void;
}
