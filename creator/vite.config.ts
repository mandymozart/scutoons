import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { sveltekit } from '@sveltejs/kit/vite';
import { cdnFonts } from '@scutoons/tokens/vite';
import { defineConfig } from 'vite';
import paraglide from './paraglide.config.js';
import { ENGINE_PATH } from './src/lib/isolation.ts';
import ports from '../shared/dev-ports.json' with { type: 'json' };

/**
 * Cross-origin isolation in dev (#298; production: static/.htaccess, the same paths): the engine's routes get COOP + COEP,
 * so live-core's audio ring can share memory (SharedArrayBuffer; without it the engine falls back to postMessage).
 * `credentialless` keeps no-cors pictures loading. Only there: COOP cuts window.opener and COEP blocks iframes that don't
 * send COEP themselves (the Reader panel's viewer iframe) – and a document keeps its isolation until a full load, which is
 * why the engine page loads itself afresh when it arrives without. The `?worker` files and live-core's sources carry the
 * header too (a worker's own response decides its isolation).
 */
const engineIsolation = {
	name: 'scutoons-engine-isolation',
	configureServer(server: { middlewares: { use: (f: (req: { url?: string }, res: { setHeader: (k: string, v: string) => void }, next: () => void) => void) => void } }) {
		server.middlewares.use((req, res, next) => {
			const url = req.url ?? '';
			const path = url.split('?')[0];
			if (ENGINE_PATH.test(path) || /[?&]worker/.test(url) || path.includes('/live-core/src/')) {
				res.setHeader('Cross-Origin-Opener-Policy', 'same-origin');
				res.setHeader('Cross-Origin-Embedder-Policy', 'credentialless');
			}
			next();
		});
	}
};

// Dev: paraglide as one module per locale instead of one per message (the dev server would otherwise serve 200–380
// message files one by one before the first render, agents/PERFORMANCE.md). Builds use the same layout (+2 KB gzip) so a
// build never swaps the modules under a running dev server (it used to break the dev apps until a restart).
export default defineConfig(({ command }) => ({
	plugins: [
		engineIsolation,
		cdnFonts(), // @font-face from the CDN (VITE_CDN_URL), the bundled copy as fallback
		sveltekit(),
		paraglideVitePlugin({ ...paraglide, outputStructure: 'locale-modules' })
	],
	server: {
		// local dev port: shared/dev-ports.json (the single source of truth)
		port: ports.creator,
		strictPort: true,
		// 127.0.0.1, not the default `localhost` (= ::1 alone on Windows; Firefox then loses Vite's HMR socket – viewer/vite.config.ts)
		host: '127.0.0.1',
		// shared workspace packages (@scutoons/*) are linked source folders outside the app (fonts, svelte files)
		fs: { allow: ['../shared'] }
	},
	preview: {
		port: ports.creator,
		strictPort: true
	}
}));
