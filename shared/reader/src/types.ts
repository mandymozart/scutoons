import type { TextBoxEffect, ShowTextBoxParams, TriggerMode } from '@scutoons/content';

/** A showTextBox event playing right now (its text box is drawn by the slice): the params + the VN player's runtime bits. */
export interface ActiveTextBox extends Omit<ShowTextBoxParams, 'wait' | 'mode'> {
	id: string;
	/** the VN sequence waits for a tap before it continues */
	wait: boolean;
	mode: TriggerMode;
	/** called when the effect has played (after delay + duration) */
	onComplete: () => void;
}

/** A showTextBox event revealing a text box right now (`Dialog` `play`): the event's timing and effect. */
export interface DialogPlay {
	/** changes when the event fires again (replays the effect) */
	key: string;
	effect?: TextBoxEffect;
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

/**
 * "Überspringen" on a player video slice (`PlayerOverlay`): a bubbling DOM event (no detail) on its `<video>`, which is
 * paused first. The app decides where it goes (the viewer: the next slice, else the follow-up bar's next step). Skipping
 * doesn't count as watched.
 */
export const SLICE_SKIP_EVENT = 'sliceskip';

export interface SlicePlayerDetail {
	state: 'play' | 'pause' | 'ended' | 'time';
	/** the video is playing (play: true; pause / ended: false) */
	playing: boolean;
	/** currentTime / duration, 0..1 (1 when ended) */
	progress: number;
}
