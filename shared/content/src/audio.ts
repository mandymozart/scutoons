import type { Slice } from './content.ts';
import { Trigger, TriggerMode } from './enums.ts';
import type { QualityVariants } from './media-quality.ts';

/**
 * A music track or a sample-library sound. `path` = the default file (the medium level, 128k – what older readers and a
 * reader without a known level play); `variants` = one file per quality level (low 96k / medium 128k / high 320k, PLAN.md
 * "RULE: one file per quality level"), toon-relative like `path`. A missing level falls back to the next lower one.
 */
export interface AudioTrack {
  id: string;
  path: string;
  variants?: QualityVariants;
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
	rect: {
		x: number;
		y: number;
		width: number;
		height: number;
	}
	mode: TriggerMode;
	exclusive: boolean;
	gate: boolean;
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
