import { svelte, vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { cdnFonts } from '@scutoons/tokens/vite';
import { defineConfig } from 'vite';
import { realpathSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import ports from '../../shared/dev-ports.json' with { type: 'json' };
import { handle } from './api.mjs';

// The storyboard studio: a local Vite + Svelte app (no SvelteKit – one page) with its API as middleware (api.mjs:
// the storyboard files, frames, variants, reviews, the scripts). Port: shared/dev-ports.json "studio".
const here = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
	// started from anywhere (npm run dev:studio, the preview): the app folder is the root
	root: here,
	plugins: [
		cdnFonts(),
		svelte({ preprocess: vitePreprocess() }),
		{ name: 'storyboard-api', configureServer(server) { server.middlewares.use(handle); } }
	],
	// one three.js for the app and the renderer (tools/storyboard-render)
	resolve: { dedupe: ['three'] },
	server: {
		port: ports.studio,
		strictPort: true,
		// the shared packages and the renderer are source folders outside the app
		// (and wherever node_modules really points: a second checkout may share the first one's – a junction)
		fs: { allow: [resolve(here, '../..'), resolve(realpathSync(resolve(here, '../../node_modules')), '..')] }
	}
});
