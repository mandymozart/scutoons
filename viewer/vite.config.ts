import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { sveltekit } from '@sveltejs/kit/vite';
import { cdnFonts } from '@scutoons/tokens/vite';
import { defineConfig } from 'vite';
import paraglide from './paraglide.config.js';
import ports from '../shared/dev-ports.json' with { type: 'json' };

/**
 * The Live pages cross-origin isolated in dev too (#198, shared/live-core README "Production headers" – production:
 * static/.htaccess): /live/… documents and the bundled workers get COOP + COEP, so live-core's audio ring can share
 * memory. `credentialless`: the page's own no-cors pictures keep loading.
 */
const liveIsolation = {
	name: 'scutoons-live-isolation',
	configureServer(server: { middlewares: { use: (f: (req: { url?: string }, res: { setHeader: (k: string, v: string) => void }, next: () => void) => void) => void } }) {
		server.middlewares.use((req, res, next) => {
			const path = (req.url ?? '').replace(/^\/(de|en)(?=\/)/, '');
			if (path.startsWith('/live/') || /[?&]worker/.test(path) || path.includes('/live-core/src/')) {
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
	// the service worker's kill switch (agents/research/PWA.md "Kill switch"): a build with VITE_SW=off registers no
	// worker, and its service-worker.js removes itself and its caches from every device that had one. Also seen by the
	// service worker's own build (SvelteKit passes `define` on).
	define: { __SW_OFF__: JSON.stringify(process.env.VITE_SW === 'off') },
	plugins: [
		liveIsolation,
		cdnFonts(), // @font-face from the CDN (VITE_CDN_URL), the bundled copy as fallback
		sveltekit(),
		paraglideVitePlugin({ ...paraglide, outputStructure: 'locale-modules' })
	],
	server: {
		// local dev port: shared/dev-ports.json (the single source of truth)
		port: ports.viewer,
		strictPort: true,
		// shared workspace packages (@scutoons/*) are linked source folders outside the app (fonts, svelte files)
		fs: { allow: ['../shared'] }
	},
	preview: {
		port: ports.viewer,
		strictPort: true
	}
}));
