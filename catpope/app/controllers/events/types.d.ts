import { Character } from '../../models/visual-novel';
import {
  VNShowMessageEvent,
  VNShowChoicesEvent,
  VNAddChoiceEvent,
  VNJoinSceneEvent,
  VNLeaveSceneEvent
} from '../../models/visual-novel';
import { IScutoon } from '../../types';

export interface IVisualNovelEvents {
  scutoon: IScutoon;
  events: any[];
  choicesContainer: HTMLElement | null;
  charactersContainer: HTMLElement | null;
  characters: Map<string, Character>;
  activeCharacters: Set<string>;
  pendingChoices: any[];
  showMessage(event: VNShowMessageEvent): Promise<void>;
  showChoices(event: VNShowChoicesEvent): Promise<void>;
  addChoice(event: VNAddChoiceEvent): Promise<void>;
  joinScene(event: VNJoinSceneEvent): Promise<void>;
  leaveScene(event: VNLeaveSceneEvent): Promise<void>;
  buildUI(): void;
  cleanup(): void;
}
