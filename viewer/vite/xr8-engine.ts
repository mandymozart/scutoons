import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import type { Plugin } from 'vite';

const FILES: Record<string, string> = { 'xr.js': 'dist/xr.js', 'xr-tracking.js': 'dist/xr-tracking.js', LICENSE: 'LICENSE' };
const BASE = '/xr8/';

/** the package's folder: the app's own node_modules, else the workspace's (hoisted) */
function engineFolder(): string {
	const found = ['../node_modules/@8thwall/engine/', '../../node_modules/@8thwall/engine/']
		.map((path) => fileURLToPath(new URL(path, import.meta.url)))
		.find((folder) => existsSync(folder));
	if (!found) throw new Error('@8thwall/engine is not installed (npm install)');
	return found;
}

/**
 * The 8th Wall engine (`@8thwall/engine`, MIT: the XR mode's image tracking) is a classic script plus a chunk it
 * imports, so it is served as it is from /xr8/ and never bundled; MIT asks for its LICENSE beside the copies.
 */
export function xr8Engine(): Plugin {
	const read = (name: string) => readFileSync(engineFolder() + FILES[name]);
	return {
		name: 'scutoons-xr8-engine',
		configureServer(server) {
			server.middlewares.use((req, res, next) => {
				const path = (req.url ?? '').split('?')[0];
				const name = path.startsWith(BASE) ? path.slice(BASE.length) : '';
				if (!Object.hasOwn(FILES, name)) return next();
				res.setHeader('Content-Type', name.endsWith('.js') ? 'text/javascript' : 'text/plain');
				res.end(read(name));
			});
		},
		generateBundle(options) {
			// SvelteKit builds twice (server, client): the files belong to the client's output only
			if (options.dir?.replaceAll('\\', '/').includes('/output/server')) return;
			for (const name of Object.keys(FILES)) this.emitFile({ type: 'asset', fileName: BASE.slice(1) + name, source: read(name) });
		}
	};
}
