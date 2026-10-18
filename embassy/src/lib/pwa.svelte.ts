// SCUTOONS Embassy as an installable web app (agents/research/PWA-APPS.md): the service worker's page side (update bar,
// kill switch) and the install state – @scutoons/pwa. The root layout calls `initPwa()` and draws the two bars.
import { dev } from '$app/environment';
import { Install, ToolPwa } from '@scutoons/pwa';

export const APP_NAME = 'SCUTOONS Embassy';
export const pwa = new ToolPwa('embassy');
export const install = new Install({ key: 'scutoons_embassy_install', dev });

export function initPwa() {
	void pwa.init({ dev, off: import.meta.env.VITE_SW === 'off' });
	install.init();
}
