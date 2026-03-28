import { Chapter, Group } from '../models/content';
import { IScutoon } from '../types';

export interface IViewInstance {
  scutoon: IScutoon;
  render(...args: any[]): Promise<any> | void;
  cleanup?(): Promise<void>;
}

export interface IToonView extends IViewInstance {
  render(): Promise<any>;
}

export interface IChapterView extends IViewInstance {
  currentChapter: Chapter | null;
  groups: Group[] | null;
  groupMap: Map<string, any> | null;
  groupHandler: any;
  render(chapterNumber?: number, label?: string | null): Promise<Chapter | null>;
  renderFromLabel(startLabel: string): Promise<void>;
  jumpToLabel(label: string): Promise<void>;
  cleanup(): Promise<void>;
}

export interface ISongView extends IViewInstance {
  render(songNumber: number): Promise<any>;
}

export interface IAccountView extends IViewInstance {
  render(): void;
}

export interface ICreditsView extends IViewInstance {
  render(): Promise<void>;
}

export interface IMembershipRequiredView extends IViewInstance {
  render(chapterNumber?: number | null, songNumber?: number | null): void;
}
