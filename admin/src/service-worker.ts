/// <reference types="@sveltejs/kit" />
/// <reference no-default-lib="true"/>
/// <reference lib="esnext" />
/// <reference lib="webworker" />
// The SCUTOONS Admin service worker (agents/research/PWA-APPS.md §2): installable + the app shell, no content cache, the team's push –
// @scutoons/pwa tool-worker.ts. Kill switch: a build with VITE_SW=off removes it and its caches.
import { build, files, version } from '$service-worker';
// the worker entry: no Svelte components in a service worker's bundle
import { startToolWorker } from '@scutoons/pwa/worker';

startToolWorker(self as unknown as Parameters<typeof startToolWorker>[0], { app: 'admin', build, files, version, off: import.meta.env.VITE_SW === 'off' });
