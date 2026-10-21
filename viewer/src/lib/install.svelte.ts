// The reader's "Install SCUTOONS as an app" (agents/research/PWA.md "Install prompt"): the shared Install state
// (@scutoons/pwa, rules in its install-rules.ts) with the reader's key; the bar is components/install-prompt.svelte, the
// how-to the /app page (#159, no bar there nor on the account page's "App"); a chapter's end bar is the milestone (next-bar.svelte).
import { dev } from '$app/environment';
import { Install } from '@scutoons/pwa';

export const install = new Install({ key: 'scutoons_install', quietPath: /\/(account\/offline|app)\/?$/, dev });
