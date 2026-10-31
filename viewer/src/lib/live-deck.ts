// The browser deck in the reader (#198, /live/deck – owner 2026-10-31: "I'd rather ship it in the reader … in the
// PWA"): what the route reads from its address and the session. The deck itself is @scutoons/live-deck.
import type { Me } from '@scutoons/content';

/** the character a deck wears when nothing else says one */
export const DEFAULT_CHARACTER = 'yellowmouse';
/** where the device remembers the last character (a guest's choice) */
export const CHARACTER_KEY = 'scutoons.deck.character';
/** where the browser deck's wasm lives (viewer/static/live/, `npm run build:wasm -w shared/live-deck`) */
export const WASM_BASE = '/live/';

/**
 * The relay from the address (no production relay yet – owner question #198): `?relay=ws://host:port/live/CODE`, or
 * `?relay=ws://host:port` + `?code=CODE`. Anything else: none (the deck plays alone).
 */
export function relayOf(search: URLSearchParams): string | undefined {
	const relay = search.get('relay')?.trim();
	if (!relay || !/^wss?:\/\//.test(relay)) return undefined;
	if (/\/live\/[^/?#]+\/?$/.test(relay)) return relay;
	const code = search.get('code')?.trim();
	return code ? `${relay.replace(/\/+$/, '')}/live/${encodeURIComponent(code)}` : undefined;
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
