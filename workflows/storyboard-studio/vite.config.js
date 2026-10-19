import { svelte, vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { cdnFonts } from '@scutoons/tokens/vite';
import { defineConfig } from 'vite';
import { existsSync, readFileSync, realpathSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import ports from '../../shared/dev-ports.json' with { type: 'json' };
import { handle } from './api.mjs';
import { handheldRelay } from './handheld-relay.mjs';

// The storyboard studio: a local Vite + Svelte app (no SvelteKit – one page) with its API as middleware (api.mjs:
// the storyboard files, frames, variants, reviews, the scripts). Port: shared/dev-ports.json "studio".
const here = dirname(fileURLToPath(import.meta.url));
// The phone as a handheld camera (README "Handheld camera"): by default the studio stays on 127.0.0.1 and the phone comes
// over USB (Chrome port forwarding → its localhost, a secure context). STUDIO_LAN=1 listens on the LAN (Wi-Fi; other
// machines get only GETs + the socket – api.mjs), STUDIO_HTTPS=1 serves .cert/key.pem + cert.pem (mkcert / openssl).
const lan = process.env.STUDIO_LAN === '1';
const cert = resolve(here, '.cert');
const https = process.env.STUDIO_HTTPS === '1' && existsSync(resolve(cert, 'key.pem')) ? { key: readFileSync(resolve(cert, 'key.pem')), cert: readFileSync(resolve(cert, 'cert.pem')) } : undefined;

export default defineConfig({
	// started from anywhere (npm run dev:studio, the preview): the app folder is the root
	root: here,
	plugins: [
		cdnFonts(),
		svelte({ preprocess: vitePreprocess() }),
		{ name: 'storyboard-api', configureServer(server) { server.middlewares.use(handle); handheldRelay(server); } }
	],
	// one three.js for the app and the renderer (tools/storyboard-render)
	resolve: { dedupe: ['three'] },
	server: {
		port: ports.studio,
		strictPort: true,
		host: lan ? '0.0.0.0' : undefined,
		https,
		// the shared packages and the renderer are source folders outside the app
		// (and wherever node_modules really points: a second checkout may share the first one's – a junction)
		fs: { allow: [resolve(here, '../..'), resolve(realpathSync(resolve(here, '../../node_modules')), '..')] }
	}
});
