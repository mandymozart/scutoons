// The reader's Web Push (agents/research/PWA.md §2b, FEATURES 10.74 phase 4): the account page's "Notifications" switch –
// the shared PushSwitch (@scutoons/pwa) for the app `viewer`. The only message is "a new chapter is out", sent by an
// admin (Domain\Push\NewChapterPush) – never automatically.
import { PushSwitch } from '@scutoons/pwa';
import { api } from '$lib/api.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import { install } from '$lib/install.svelte.ts';


export const push = new PushSwitch({ app: 'viewer', api, locale: getLocale, installFirst: () => install.iosHowTo });
