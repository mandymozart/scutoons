import type { Onomatopoeia, TriggerMode } from '@scutoons/content';

/** An onomatopoeia on screen: the config plus the runtime bits the VN player (the app) attaches. */
export interface ActiveOnomatopoeia extends Onomatopoeia {
	id: string;
	/** the VN sequence waits for a tap before it continues */
	wait: boolean;
	mode: TriggerMode;
	/** called when the effect has played (after delay + duration) */
	onComplete: () => void;
}

/** A rect in % of the slice box (top-left origin), like dialog / trigger / sample rects. */
export interface PercentRect {
	x: number;
	y: number;
	width: number;
	height: number;
}
