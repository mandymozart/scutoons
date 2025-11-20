export interface Character {
  name: string;
  image: string;
  description?: string;
  collection?: string;
  avatar?: string;
  moods?: Record<string, string>;
}

export interface VNCharacterRef {
  name: string;
  position?: {
    variant: "avatar" | "full" | "portrait";
    relativeToMessageBox: "front" | "back";
    order: number;
  };
  mood?: string;
}

export interface VNVariableSetter {
  set: {
    variable: string;
    value: string;
  };
}

export interface VNVariableAdder {
  add: {
    variable: string;
    value: number;
  };
}

export interface VNShowMessageEvent {
  type: "showMessage";
  params: {
    text: string;
    character: string;
    wait?: boolean;
  };
}

export interface VNShowChoicesEvent {
  type: "showChoices";
  params?: {};
}

export interface VNAddChoiceEvent {
  type: "addChoice";
  params: {
    label: string;
    onSelect?: VNVariableSetter | VNVariableAdder;
  };
}

export interface VNJoinSceneEvent {
  type: "joinScene";
  params: {
    characters: VNCharacterRef[];
  };
}

export interface VNLeaveSceneEvent {
  type: "leaveScene";
  params: {
    characters: VNCharacterRef[];
  };
}

export interface VNEndEvent {
  type: "end";
  params: {
    jumpTo: string;
  };
}

export type VNEvent =
  | VNShowMessageEvent
  | VNShowChoicesEvent
  | VNAddChoiceEvent
  | VNJoinSceneEvent
  | VNLeaveSceneEvent
  | VNEndEvent;
