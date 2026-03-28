import { Renderer, Language, Status, Interaction } from './enums';
import { AudioEvents, AudioConfiguration } from './audio';
import { VNEvent } from './visual-novel';

export interface KeyVisual {
  image: string;
  video?: string;
}

export interface Dialog {
  rect: {
    x: number;
    y: number;
    width: number;
  };
  text: string[];
}

export interface Onomatopoeia {
  position: {
    x: number;
    y: number;
    size: number;
    align: "left" | "right" | "center";
  };
  rotation: number;
  text: string[];
}

export interface Slice {
  image: string;
  audio?: AudioEvents;
  dialog?: Dialog[];
  onomatopoeia?: Onomatopoeia[];
  _type?: string;
  _label?: string;
}

export interface Group {
  label: string;
  renderer: Renderer;
  interaction?: Interaction;
  slices?: Slice[];
  events?: VNEvent[];
}

export interface GroupRef {
  src: string;
}

export interface Chapter {
  title: string;
  author: string;
  number: number;
  language: Language;
  release: string;
  status: Status;
  description: string;
  thumbnail: string;
  keyvisual?: KeyVisual;
  audio?: AudioConfiguration;
  groups: (Group | GroupRef)[];
}

export interface ChapterRef {
  src: string;
}

export interface Song {
  title: string;
  author?: string;
  number: number;
  language: Language;
  release: string;
  status: Status;
  description: string;
  thumbnail: string;
  src: string;
}

export interface SongWithId extends Song {
  id: number;
}

export interface Cover {
  square?: string;
  portrait?: string;
}

export interface Credits {
  artist?: string[];
  writer?: string;
  music?: string[];
  sound?: string[];
}

export interface Person {
  name: string;
  roles: string[];
}

export interface PerformanceCast {
  name: string;
  venue: string;
  date: string;
  conductor?: string;
  members: Person[];
}

export interface SongCredits {
  title: string;
  credits: Person[];
}

export interface FullCredits {
  general: Person[];
  songs: SongCredits[];
  performances: PerformanceCast[];
  specialThanks: string[];
}

export interface SequenceItem {
  type: "song" | "chapter";
  file: string;
}

export interface Season {
  title: string;
  authors?: string[];
  language?: string;
  number?: number;
  release?: string;
  description?: string;
  cover?: Cover;
  credits?: Credits;
  sequence: SequenceItem[];
}

export interface Download {
  complete: string | null;
  music: string;
}

export interface Toon {
  title: string;
  authors: string[];
  chapters: number;
  language: string;
  release: string;
  status: Status;
  description: string;
  thumbnail: string;
  cover: Cover;
  download: Download;
}

export type LoadedSequenceItem = (Chapter | Song) & {
  itemType: 'chapter' | 'song';
  itemNumber: number;
};
