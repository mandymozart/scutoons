import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import paraglide from './paraglide.config.js';
import ports from '../shared/dev-ports.json' with { type: 'json' };

// Dev: paraglide as one module per locale instead of one per message (the dev server would otherwise serve 200–380
// message files one by one before the first render, agents/PERFORMANCE.md). Builds use the same layout (+2 KB gzip) so a
// build never swaps the modules under a running dev server (it used to break the dev apps until a restart).
export default defineConfig(({ command }) => ({
	plugins: [
		sveltekit(),
		paraglideVitePlugin({ ...paraglide, outputStructure: 'locale-modules' })
	],
	server: {
		// local dev port: shared/dev-ports.json (the single source of truth)
		port: ports.shop,
		strictPort: true,
		// shared workspace packages (@scutoons/*) are linked source folders outside the app (fonts, svelte files)
		fs: { allow: ['../shared'] }
	},
	preview: {
		port: ports.shop,
		strictPort: true
	}
}));
