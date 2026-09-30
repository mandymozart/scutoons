import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import paraglide from './paraglide.config.js';

// Dev: paraglide as one module per locale instead of one per message (the dev server would otherwise serve 200–380
// message files one by one before the first render, agents/PERFORMANCE.md); builds keep per-message modules (tree-shaking).
export default defineConfig(({ command }) => ({
	plugins: [sveltekit(), paraglideVitePlugin({ ...paraglide, outputStructure: command === 'serve' ? 'locale-modules' : 'message-modules' })],
	server: {
		// local dev ports (agents/PLATFORM.md): viewer 8000, cdn 8001, server 9000, creator 10000, admin 11000
		port: 11000,
		strictPort: true,
		// shared workspace packages (@scutoons/*) are linked source folders outside the app (fonts, svelte files)
		fs: { allow: ['../shared'] }
	},
	preview: {
		port: 11000,
		strictPort: true
	}
}));
