import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { readFileSync } from 'node:fs';
import { gitSha } from '../deploy/version.mjs';

const pkg = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8'));
// one build time per build: the config is loaded more than once (app, service worker) – all must agree on the name
process.env.SCUTOONS_BUILD_ID ??= Date.now().toString(36);

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://svelte.dev/docs/kit/integrations
	// for more information about preprocessors
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({ fallback: 'index.html' }),
		// the build's name = the service worker's cache name (agents/research/PWA.md): package version + the git sha
		// version.json reports (deploy/version.mjs) + the build time, so every build is a new version
		version: { name: `${pkg.version}-${gitSha().slice(0, 12)}-${process.env.SCUTOONS_BUILD_ID}` },
		// src/service-worker.ts is registered by lib/pwa.svelte.ts (the update prompt, the VITE_SW=off kill switch);
		// static/ai (the MCP download) is never precached
		serviceWorker: { register: false, files: (file) => !/^ai\/|\.DS_Store/.test(file) }
	}
};

export default config;
