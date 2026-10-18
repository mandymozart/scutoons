// The simulated reader screens (agents/DOCKING.md "The Reader panel", research/COLLAGE.md §4.4): the dock's Reader frame
// and the score stage show a phone or a desktop window at the device's own size, then scale it to their room.
import { DEFAULT_ZOOM, PHONE_ZOOM } from '@scutoons/reader/score-anchors';

export type Screen = 'phone' | 'desktop';

/**
 * the emulated screens in CSS px: a frame's viewport must be exactly the device's, whatever the creator's own root size
 * (the reader lays out by its px breakpoints, a score's pages by the sheet's px) – then scaled to the panel (a device
 * size, not a UI size)
 */
export const SCREENS: Record<Screen, { width: number; height: number }> = { phone: { width: 390, height: 844 }, desktop: { width: 1440, height: 810 } };

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

/** the zoom a reader's score view starts with on that screen (narrower than 40rem at the 16px root: the phone zoom) */
export const screenZoom = (screen: Screen): number => (SCREENS[screen].width / 16 < 40 ? PHONE_ZOOM : DEFAULT_ZOOM);
