// The simulated reader screens (agents/DOCKING.md "The Reader panel", research/COLLAGE.md §4.4): the dock's Reader frame
// and the score stage show a phone or a desktop window at the device's own size, then scale it to their room.
import { DEFAULT_ZOOM, PHONE_ZOOM } from '@scutoons/reader/score-anchors';

export type Screen = 'phone' | 'desktop';

/**
 * An emulated screen in CSS px: a frame's viewport must be exactly the device's, whatever the creator's own root size
 * (the reader lays out by its px breakpoints, a score's pages by the sheet's px) – then scaled to the panel (a device
 * size, not a UI size). `root` = the reader's root font size there (shared/tokens/base.css: 14px < 480px, 24px ≥ 768px),
 * `touch` = a coarse pointer (tokens.css: controls at least 44px high).
 */
export interface ScreenSize {
	width: number;
	height: number;
	root: number;
	touch: boolean;
}

export const SCREENS: Record<Screen, ScreenSize> = {
	phone: { width: 390, height: 844, root: 14, touch: true },
	desktop: { width: 1440, height: 810, root: 24, touch: false }
};

/** the rem sizes of shared/tokens/tokens.css that the reader's score view reads (its header, footer, labels), in rem */
const READER_REM: Record<string, number> = {
	'--space-025': 0.25,
	'--space-05': 0.5,
	'--space-075': 0.75,
	'--space-1': 1,
	'--space-15': 1.5,
	'--space-3': 3,
	'--type-body-size': 1,
	'--type-button-size': 1,
	'--type-label-size': 1,
	'--type-caption-size': 0.75
};
/** tokens.css's control floor on a coarse pointer (the touch-target minimum) */
const TOUCH_FLOOR = 44;

/**
 * The reader's sizes on `screen`, in its CSS px (research/COLLAGE.md §4.4): the creator's root is not the reader's
 * (tools.css: 24px × 0.7 from 768px), so the tokens' rem inside a frame would size the score view's header and footer –
 * and with them the sheet, which is Verovio's page – unlike a reader's. Set on the frame, they give the view inside the
 * reader's spacing, type and control height there (device sizes, like the frame's own px – not UI sizes).
 */
export function readerSizes(screen: ScreenSize): Record<string, string> {
	const px = (rem: number) => `${rem * screen.root}px`;
	const sizes = Object.fromEntries(Object.entries(READER_REM).map(([name, rem]) => [name, px(rem)]));
	const control = `calc(${px(2)} + 2 * var(--border-width-regular))`;
	return { ...sizes, '--control-height': screen.touch ? `max(${TOUCH_FLOOR}px, ${control})` : control };
}

/** how much of `room` (px) a screen may fill: the largest scale that fits both ways (0 while the room is unknown) */
export function screenScale(screen: { width: number; height: number }, room: { width: number; height: number }): number {
	return room.width > 0 && room.height > 0 ? Math.min(room.width / screen.width, room.height / screen.height) : 0;
}

/** the screen picked last under `key` (a per-device convenience – RULES "localStorage"); a phone by default */
export function storedScreen(key: string): Screen {
	try {
		return localStorage.getItem(key) === 'desktop' ? 'desktop' : 'phone';
	} catch {
		return 'phone';
	}
}

export function storeScreen(key: string, screen: Screen): void {
	try {
		localStorage.setItem(key, screen);
	} catch {
		// storage blocked: the pick lasts for this page view
	}
}

/** the zoom a reader's score view starts with on that screen (narrower than 40rem at the reader's root: the phone zoom) */
export const screenZoom = (screen: Screen): number => (SCREENS[screen].width / SCREENS[screen].root < 40 ? PHONE_ZOOM : DEFAULT_ZOOM);
