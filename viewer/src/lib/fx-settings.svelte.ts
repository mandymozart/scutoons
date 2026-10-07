import { configureFx } from '@scutoons/reader';
import { Howler } from 'howler';

/**
 * The account page's "Sound effects" volume and "Vibration" switch (agents/FX.md): kept on this device (localStorage) –
 * a phone and a laptop want different levels. Sound is part of a scutoon: there is no "off", only a volume.
 * The effect sounds go through Howler's master gain, so the music's mute applies to them as well.
 */

const VOLUME_KEY = 'scutoons_fx_volume';
const HAPTICS_KEY = 'scutoons_fx_haptics';

function read(key: string): string | null {
	try {
		return localStorage.getItem(key);
	} catch {
		return null;
	}
}

function write(key: string, value: string) {
	try {
		localStorage.setItem(key, value);
	} catch {
		/* private window: this visit only */
	}
}

function createFxSettings() {
	const storedVolume = Number(read(VOLUME_KEY));
	let volume = $state(read(VOLUME_KEY) !== null && Number.isFinite(storedVolume) ? Math.min(1, Math.max(0, storedVolume)) : 0.8);
	let haptics = $state(read(HAPTICS_KEY) !== '0');

	return {
		get volume() {
			return volume;
		},
		get haptics() {
			return haptics;
		},
		/** once, in the root layout: the routing and the stored values */
		init() {
			configureFx({
				output: () => (Howler.ctx && Howler.masterGain ? { ctx: Howler.ctx, destination: Howler.masterGain } : null),
				volume,
				haptics
			});
		},
		setVolume(value: number) {
			volume = Math.min(1, Math.max(0, value));
			write(VOLUME_KEY, String(volume));
			configureFx({ volume });
		},
		setHaptics(value: boolean) {
			haptics = value;
			write(HAPTICS_KEY, value ? '1' : '0');
			configureFx({ haptics });
		}
	};
}

export const fxSettings = createFxSettings();
