import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import type { Plugin } from 'vite';

/** the stage's own files, as OSCT's editor ships them (shared/xr-engine/osct/editor/static, with their READMEs) */
const FILES: Record<string, string> = {
	'models/desk-eiermann.glb': 'model/gltf-binary',
	'textures/plywood_diff_1k.jpg': 'image/jpeg',
	'textures/plywood_rough_1k.jpg': 'image/jpeg',
	'hdri/wooden_studio_17_1k.hdr': 'application/octet-stream'
};
const BASE = '/xr-stage/';
const FOLDER = fileURLToPath(new URL('../../shared/xr-engine/osct/editor/static/', import.meta.url));

/** The XR stage's desk, its textures and its sky at /xr-stage/ (src/lib/editor/xr-stage.ts): they stay with the engine's copy, so the app serves them from there. */
export function xrStageFiles(): Plugin {
	const read = (name: string) => readFileSync(FOLDER + name);
	return {
		name: 'scutoons-xr-stage-files',
		configureServer(server) {
			server.middlewares.use((req, res, next) => {
				const path = (req.url ?? '').split('?')[0];
				const name = path.startsWith(BASE) ? path.slice(BASE.length) : '';
				if (!Object.hasOwn(FILES, name)) return next();
				res.setHeader('Content-Type', FILES[name]);
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
