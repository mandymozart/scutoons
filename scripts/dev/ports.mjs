// The local dev ports for Node scripts: shared/dev-ports.json (the single source of truth), without its "$comment".
//   import { PORTS, local } from './dev/ports.mjs';   local('api', '/api') → 'http://localhost:9000/api'
import { readFileSync } from 'node:fs';

const file = JSON.parse(readFileSync(new URL('../../shared/dev-ports.json', import.meta.url), 'utf8'));

/** @type {Record<string, number>} service name → port */
export const PORTS = Object.fromEntries(Object.entries(file).filter(([, port]) => typeof port === 'number'));

/** `http://localhost:<port of name><path>`; throws for an unknown service name */
export function local(name, path = '') {
	if (!(name in PORTS)) throw new Error(`unknown dev service "${name}" (shared/dev-ports.json)`);
	return `http://localhost:${PORTS[name]}${path}`;
}
