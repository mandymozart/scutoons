// SCUTOONS Creator as an installable web app (agents/research/PWA-APPS.md): the service worker's page side (update bar,
// kill switch), the install state and the push switch of the "Benachrichtigungen" page (routes/notifications, the team's
// kinds – research/PWA-APPS.md §5) – @scutoons/pwa. The root layout calls `initPwa()` and draws the two bars.
import { dev } from '$app/environment';
import { Install, PushSwitch, ToolPwa } from '@scutoons/pwa';
import { api } from '$lib/api.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import { shared } from '$lib/utils/shared.ts';

export const APP_NAME = 'SCUTOONS Creator';
export const pwa = shared('pwa', () => new ToolPwa('creator'));
export const install = shared('install', () => new Install({ key: 'scutoons_creator_install', dev }));

export function initPwa() {
	void pwa.init({ dev, off: import.meta.env.VITE_SW === 'off' });
	install.init();
}
export const push = shared('push', () => new PushSwitch({ app: 'creator', api, locale: getLocale, installFirst: () => install.iosHowTo }));
