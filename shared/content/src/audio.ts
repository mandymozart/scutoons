import type { Slice } from './content.ts';
import { Trigger, TriggerMode } from './enums.ts';

export interface AudioTrack {
  id: string;
  path: string;
}

export interface MusicEventTrack {
  id: string;
  volume: number;
	// pan?: number;      // Future
  // pitch?: number;    // Future
  // fadeTime?: number; // Future
}

export interface MusicEvent {
  tracks: MusicEventTrack[];
}

export interface SampleTrigger {
  type: Trigger;
	/** hit area in % of the slice; none = the whole slice */
	rect?: {
		x: number;
		y: number;
		width: number;
		height: number;
	}
	mode: TriggerMode;
	/** mute other audio (default false) */
	exclusive?: boolean;
	/** only play if not already playing (default false) */
	gate?: boolean;
}

export interface Sample {
	id: string;
  trigger?: SampleTrigger; // evaluate if this is needed or can be handled by the new visual novel event system?
	volume?: number;
	delay?: number;
	loop?: boolean;
	repeat?: number; // not implemented
}

export interface AudioEvent {
  samples: Sample[];
  // synths, effects, spatialisation, etc.
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

export interface AudioEventsConfig {
	events?: AudioEvents;
	index: number;
	slices: Slice[];
}
