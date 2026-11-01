// The browser deck in the reader (#198, /live/deck – owner 2026-10-31: "I'd rather ship it in the reader … in the
// PWA"): what the route reads from its address and the session. The deck itself is @scutoons/live-deck.
import type { Me } from '@scutoons/content';

/** the character a deck wears when nothing else says one */
export const DEFAULT_CHARACTER = 'yellowmouse';
/** where the device remembers the last character (a guest's choice) */
export const CHARACTER_KEY = 'scutoons.deck.character';
/** where the browser deck's wasm lives (viewer/static/live/, `npm run build:wasm -w shared/live-deck`) */
export const WASM_BASE = '/live/';

/** the session a deck joins when the address names none (#199: the Cathedral's for now) */
export const DEFAULT_CODE = 'CATH';

/**
 * The session code (#199): the path's `/live/deck/<code>`, else `?code=`, else DEFAULT_CODE.
 */
export function codeOf(search: URLSearchParams, pathCode?: string): string {
	return pathCode?.trim() || search.get('code')?.trim() || DEFAULT_CODE;
}

/**
 * The relay the deck joins (#199, owner 2026-10-31: "the relay needs a fixed, pre-built socket address"): the hosted
 * one, `<live>/live/<code>` – `live` = LIVE_URL (`VITE_LIVE_URL`, default wss://live.scutoons.com). `?relay=` overrides
 * it (a local relay): a whole session URL `?relay=ws://host:port/live/CODE`, or a host and the code as above. A
 * `?relay=` that is no ws:// / wss:// URL is ignored.
 */
export function relayOf(search: URLSearchParams, live: string, pathCode?: string): string {
	const code = encodeURIComponent(codeOf(search, pathCode));
	const relay = search.get('relay')?.trim();
	if (relay && /^wss?:\/\/[^/]/.test(relay)) {
		return /\/live\/[^/?#]+\/?$/.test(relay) ? relay : `${relay.replace(/\/+$/, '')}/live/${code}`;
	}
	return `${live.replace(/\/+$/, '')}/live/${code}`;
}

/** The deck's character: `?character=`, else the member's avatar body, else the device's last one, else the default. */
export function characterOf(search: URLSearchParams, me: Me | null, stored: string | null): string {
	return search.get('character')?.trim() || me?.avatar?.body?.character || stored || DEFAULT_CHARACTER;
}

/** The deck's name at the relay: the member's name, else the guest default (at most 24 characters). */
export function nickOf(me: Me | null, guest: string): string {
	return (me?.user?.display_name?.trim() || guest).slice(0, 24);
}

/** WebGL2 only (`?gl=1`) – e.g. to compare it with WebGPU on one device */
export const preferOf = (search: URLSearchParams): 'auto' | 'webgl2' => (search.get('gl') === '1' ? 'webgl2' : 'auto');

/**
 * Who may join (owner 2026-10-31: "Everyone sees the route for now. We can tie it to roles or events later."): the one
 * place a gate goes later – a role, an event's invite. Today: everyone, guests included.
 */
export function mayJoin(me: Me | null): boolean {
	void me;
	return true;
}

/**
 * Play the character's rig instead of the fixed test patch (#191, research/INSTRUMENTS.md §7): `?rig=1`, while the
 * phone check is open – then it becomes the default for every character with a rig.
 */
export const rigOf = (search: URLSearchParams): boolean => search.get('rig') === '1';

/** a slider's range and label (@scutoons/live-deck's `Spec`) */
type Spec = { min: number; max: number; value: number; log?: boolean; label: string };

/** the controls panel's layout: the bar's quick sliders, the pads (x / y pairs + their label's key), the rest */
export interface PanelLayout {
	all: Record<string, Spec>;
	quick: string[];
	pads: { x: string; y: string; label: 'voice' | 'incense' }[];
	sliders: [string, Spec][];
}

/**
 * The panel for the deck's sliders: the fixed patch keeps its layout (the voice, the drone, the melody's spread in
 * the bar; the voice and incense pads); a rig's knobs put its first three gains in the bar and the rest in the fold –
 * no pads (which pair a rig's knobs make is the mapping's job, #209) and no melody (the page doesn't play it there).
 */
export function panelLayout(params: Record<string, Spec>, melody: Record<string, Spec>): PanelLayout {
	if ('voice.ratio' in params && 'incense.cutoff' in params) {
		const all = { ...melody, ...params };
		const quick = ['voice.gain', 'drone.gain', 'melody.spread'];
		const pads = [
			{ x: 'voice.ratio', y: 'voice.index', label: 'voice' as const },
			{ x: 'incense.cutoff', y: 'sway.rate', label: 'incense' as const }
		];
		const onPads = new Set(pads.flatMap((p) => [p.x, p.y]));
		return { all, quick, pads, sliders: Object.entries(all).filter(([p]) => !onPads.has(p) && !quick.includes(p)) };
	}
	const quick = Object.keys(params).filter((p) => p.endsWith('.gain')).slice(0, 3);
	return { all: params, quick, pads: [], sliders: Object.entries(params).filter(([p]) => !quick.includes(p)) };
}
