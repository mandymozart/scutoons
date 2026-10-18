import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { readFileSync } from 'node:fs';
import { gitSha } from '../deploy/version.mjs';

const pkg = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8'));
// one build time per build: the config is loaded more than once (app, service worker) – all must agree on the name
process.env.SCUTOONS_BUILD_ID ??= Date.now().toString(36);

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	// single-page app: every route is rendered in the browser (session + role gate), index.html as fallback
	kit: {
		adapter: adapter({ fallback: 'index.html' }),
		// the build's name = the service worker's cache name (agents/research/PWA-APPS.md): every build a new version
		version: { name: `${pkg.version}-${gitSha().slice(0, 12)}-${process.env.SCUTOONS_BUILD_ID}` },
		// src/service-worker.ts is registered by lib/pwa.svelte.ts (@scutoons/pwa ToolPwa: the update bar, VITE_SW=off)
		serviceWorker: { register: false }
	}
};

export default config;
