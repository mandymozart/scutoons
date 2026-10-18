// SCUTOONS Admin as an installable web app (agents/research/PWA-APPS.md): the service worker's page side (update bar,
// kill switch), the install state and the push switch of the "Benachrichtigungen" page (routes/notifications, the team's
// kinds – research/PWA-APPS.md §5) – @scutoons/pwa. The root layout calls `initPwa()` and draws the two bars.
import { dev } from '$app/environment';
import { Install, PushSwitch, ToolPwa } from '@scutoons/pwa';
import { api } from '$lib/api.ts';
import { getLocale } from '$lib/paraglide/runtime.js';

export const APP_NAME = 'SCUTOONS Admin';
export const pwa = new ToolPwa('admin');
export const install = new Install({ key: 'scutoons_admin_install', dev });

export function initPwa() {
	void pwa.init({ dev, off: import.meta.env.VITE_SW === 'off' });
	install.init();
}
export const push = new PushSwitch({ app: 'admin', api, locale: getLocale, installFirst: () => install.iosHowTo });
