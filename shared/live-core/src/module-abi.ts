// GENERATED from deck/module/src/abi.rs – do not edit: `UPDATE_FIXTURES=1 cargo test -p deck-module module_abi_ts` writes it,
// a plain run fails when it is stale. The module ABI's numbers (deck/module/README.md) for the browser host (modules.ts).
export const ABI_VERSION = 1;
/** the wasm custom section holding the descriptor's JSON */
export const SECTION = 'deck.module';
export const MAX_BLOCK = 1024;
export const MAX_AUDIO_PORTS = 4;
export const MAX_CONTROL_IN = 8;
export const MAX_CONTROL_OUT = 8;
export const MAX_PARAMS = 32;
export const MAX_VARS = 32;
export const MAX_EVENTS = 64;
export const MAX_PROPS = 4;
export const STR_LEN = 32;
export const VAR_KEY_LEN = 24;
export const EVENT_NAME_LEN = 16;
export const PROP_KEY_LEN = 8;
/** byte offsets of `Io`'s fields from the address `dm_io()` returns */
export const OFFSETS = {
	size: 72296,
	event: { size: 88, start: 0, len: 8, port: 16, nprops: 20, name: 24, keys: 40, values: 72 },
	tempoFrame: 0,
	tempoBeat: 8,
	bpm: 16,
	rate: 24,
	frame: 32,
	seed: 40,
	frames: 48,
	running: 52,
	perBar: 56,
	varsLen: 60,
	controlIn: 64,
	characterId: 96,
	characterPersona: 128,
	characterMood: 160,
	varKeys: 192,
	varValues: 960,
	audioIn: 1088,
	audioOut: 33856,
	controlOut: 66624,
	eventsLen: 66656,
	eventsDropped: 66660,
	events: 66664,
} as const;
