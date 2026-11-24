// The installed creator's side of the page (lib/shell/desktop.ts): a link handed to the app opens its item, and "Open in
// the deck" on a machine without a deck offers the deck's download instead. The web creator skips all of it.
import { goto } from '$app/navigation';
import { m } from '$lib/paraglide/messages.js';
import { toast } from '$lib/toasts.ts';
import { fillDeckHooks } from './deck.ts';
import { creatorLinkHref, desktopBridge } from './desktop.ts';
import { shell } from './shell.svelte.ts';
import { workspaceRow } from './workspaces.ts';

export function startDesktop(): void {
	const bridge = desktopBridge();
	if (!bridge) return;
	fillDeckHooks({
		missing: () => bridge.deckMissing(),
		offer: () =>
			toast(m['shell.deck_missing'](), {
				type: 'warning',
				...(bridge.hasDeckDownload ? { action: { label: m['shell.deck_download'](), run: () => bridge.openDeckDownload() } } : {})
			})
	});
	bridge.onLink((link) => {
		const href = creatorLinkHref(link, (kind) => workspaceRow(shell.use, shell.state.tabs).find((entry) => entry.kind === kind)?.href ?? null);
		if (href) void goto(href);
	});
}
