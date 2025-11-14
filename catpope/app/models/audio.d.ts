import { Trigger } from './enums';

export interface AudioTrack {
  id: string;
  path: string;
}

export interface MusicEventTrack {
  id: string;
  volume: number;
}

export interface MusicEvent {
  tracks: MusicEventTrack[];
}

export interface SampleTrigger {
  type: Trigger;
}

export interface Sample {
  id: string;
  trigger: SampleTrigger;
}

export interface AudioEvent {
  samples: Sample[];
}

export interface AudioEvents {
  music?: MusicEvent;
  audio?: AudioEvent;
}

export interface AdoptiveMusicPlayerConfiguration {
  transitionDuration?: number;
  defaultTrack?: string;
  tracks?: AudioTrack[];
}

export interface AudioConfiguration {
  music?: AdoptiveMusicPlayerConfiguration;
  sampleLibrary?: AudioTrack[];
}
