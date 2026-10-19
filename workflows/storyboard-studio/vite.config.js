import { svelte, vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { cdnFonts } from '@scutoons/tokens/vite';
import { defineConfig } from 'vite';
import { realpathSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import ports from '../../shared/dev-ports.json' with { type: 'json' };
import { handle } from './api.mjs';
import { handheldRelay } from './handheld-relay.mjs';
import { startPhoneServer } from './phone-https.mjs';

// The storyboard studio: a local Vite + Svelte app (no SvelteKit – one page) with its API as middleware (api.mjs:
// the storyboard files, frames, variants, reviews, the scripts). Port: shared/dev-ports.json "studio".
const here = dirname(fileURLToPath(import.meta.url));
// The phone as a handheld camera (README "Handheld camera"): the studio stays on 127.0.0.1 for the PC; the phone comes over
// Wi-Fi to https://<this PC>:<phone port> (phone-https.mjs – owner 2026-10-19: "i don t want to plug in a cable"; GETs +
// the socket only) or over USB. STUDIO_LAN=0 keeps it off the LAN.
const phoneOn = process.env.STUDIO_LAN !== '0';
/** the phone's HTTPS port: the studio's + 443 (14000 → 14443) – one source, shared/dev-ports.json "studio" */
const PHONE_OFFSET = 443;

export default defineConfig({
	// started from anywhere (npm run dev:studio, the preview): the app folder is the root
	root: here,
	plugins: [
		cdnFonts(),
		svelte({ preprocess: vitePreprocess() }),
		{
			name: 'storyboard-api',
			configureServer(server) {
				server.middlewares.use(handle);
				handheldRelay(server);
				if (!phoneOn) return;
				server.httpServer?.once('listening', () => {
					// where the studio really listens (localhost may be ::1 only)
					const { address: host, port: target } = server.httpServer.address();
					const phone = startPhoneServer({ port: target + PHONE_OFFSET, host, target, certDir: resolve(here, '.cert'), log: (m) => server.config.logger.info(m) });
					// a restart (config change) closes the old one first – the new studio opens its own
					server.httpServer.once('close', () => phone?.stop());
				});
			}
		}
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
