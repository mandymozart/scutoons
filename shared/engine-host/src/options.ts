// options.ts (#298): what an app hands the host, and the small decisions the host makes from it. Pure.
import type { DeckOptions } from '@scutoons/live-deck';

export interface EngineHostOptions {
	/** the library API (scene-content, characters) – `API_URL` */
	api: string;
	/** the CDN root (portraits, the set) */
	cdn: string;
	/** where space_web.js, space_web_bg.wasm and live_core.wasm are served (`/live/`; `build:wasm` writes them there) */
	wasmBase: string;
	/** the character the deck wears (its portraits and core rig) */
	character: string;
	/** the deck's name at a relay */
	nick?: string;
	/** a relay session (`ws://host/live/CODE`); none = the deck plays and draws alone */
	relay?: string;
	/** the relay session token */
	token?: string;
	/** WebGL2 only */
	prefer?: 'auto' | 'webgl2';
	/** the 3D's set (default: the cathedral) */
	set?: string;
	/** the 3D's pixel ratio cap */
	maxScale?: number;
	/** play the character's core rig instead of the fixed patch (the viewer's `?rig=1`), also after a character switch */
	rigs?: boolean;
	/** live-core's transport: 'auto' (shared when the page is cross-origin isolated, else transfer – default) or forced */
	transport?: 'auto' | 'shared' | 'transfer';
	/** the deck's status (the relay's, the sound's) – the host logs it too */
	onStatus?: DeckOptions['onStatus'];
	/** the answers to the conductor's scene pushes */
	onSynced?: DeckOptions['onSynced'];
}

/** a folder URL ends with `/` */
export const folder = (url: string): string => (url.endsWith('/') ? url : `${url}/`);

/** the options the deck gets from the host's: no relay → none, a nick of at most 24 characters */
export function deckOptions(o: EngineHostOptions, onStatus: (s: string) => void): DeckOptions {
	return {
		relay: o.relay || undefined,
		character: o.character,
		nick: (o.nick?.trim() || 'deck').slice(0, 24),
		token: o.token,
		api: o.api,
		cdn: folder(o.cdn),
		onStatus,
		onSynced: o.onSynced,
		// #366: the relay client from the host's wasm folder (loaded when the deck joins)
		wasm: folder(o.wasmBase)
	};
}
