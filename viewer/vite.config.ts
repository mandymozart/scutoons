import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [
		sveltekit(),
		paraglideVitePlugin({
			project: './project.inlang',
			outdir: './src/lib/paraglide'
		})
	],
	server: {
		// local dev ports (agents/PLATFORM.md): viewer 8000, cdn 8001, server 9000, creator 10000, admin 11000
		port: 8000,
		strictPort: true,
		// shared workspace packages (@scutoons/*) are linked source folders outside the app (fonts, svelte files)
		fs: { allow: ['../shared'] }
	}
});
