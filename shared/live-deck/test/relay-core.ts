// The relay client's core in Node (#366): relay-web's wasm as `npm run build:wasm -w shared/live-deck` (or `-- --relay`)
// writes it to shared/live-deck/wasm/ – null when it isn't built (the tests that need it skip, saying so; deck:ci builds
// it before these tests).
import { existsSync, readFileSync } from 'node:fs';
import { loadRelay, type RelayModule } from '../src/client.ts';

const dir = new URL('../wasm/', import.meta.url);
const wasm = new URL('relay_web_bg.wasm', dir);

/** why the core is not there ('' = it is) */
export const NO_CORE = existsSync(wasm) && existsSync(new URL('relay_web.js', dir)) ? '' : 'relay-web is not built (npm run build:wasm -w shared/live-deck -- --relay)';

export function relayCore(): Promise<RelayModule> {
	return loadRelay(dir.href, readFileSync(wasm));
}
