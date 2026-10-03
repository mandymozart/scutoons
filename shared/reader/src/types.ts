import type { Onomatopoeia, OnomatopoeiaEffect, TriggerMode } from '@scutoons/content';

/** An onomatopoeia on screen: the config plus the runtime bits the VN player (the app) attaches. */
export interface ActiveOnomatopoeia extends Onomatopoeia {
	id: string;
	/** the event reveals this text box instead of drawing its own onomatopoeia (`params.dialog`); the slice draws it */
	dialog?: string;
	/** the VN sequence waits for a tap before it continues */
	wait: boolean;
	mode: TriggerMode;
	/** called when the effect has played (after delay + duration) */
	onComplete: () => void;
}

/** An onomatopoeia event revealing a text box right now (`Dialog` `play`): the event's timing and effect. */
export interface DialogPlay {
	/** changes when the event fires again (replays the effect) */
	key: string;
	effect?: OnomatopoeiaEffect;
	duration?: number;
	delay?: number;
	onComplete: () => void;
}

/** A rect in % of the slice box (top-left origin), like dialog / trigger / sample rects. */
export interface PercentRect {
	x: number;
	y: number;
	width: number;
	height: number;
}

/**
 * A player video slice (`SliceVideo.player`) tells its ancestors what it does with a bubbling DOM event on its `<video>`:
 * the app pauses the chapter's music while it plays (`playing`) and records how far it was watched (`progress`, 0..1 of
 * the video; `ended` at the end). `time` events come at most once per percent.
 */
export const SLICE_PLAYER_EVENT = 'sliceplayer';

export interface SlicePlayerDetail {
	state: 'play' | 'pause' | 'ended' | 'time';
	/** the video is playing (play: true; pause / ended: false) */
	playing: boolean;
	/** currentTime / duration, 0..1 (1 when ended) */
	progress: number;
}
