// The installed creator (creator-app/, the same page in an Electron window): the bridge its preload puts on the window, and
// where a `scutoons-creator://<workspace>/<address>` link leads (server/schema/tables/deck-link.json). Pure: test/desktop.test.ts.
import { CREATOR_APP, readAppLink } from '@scutoons/api';
import { hrefOf, type WorkspaceKind } from './address.ts';
import { ROW } from './workspaces.ts';

export interface DesktopBridge {
	/** the app has the deck's download to offer */
	hasDeckDownload: boolean;
	/** the links handed to the app; those that came before the page are replayed */
	onLink(listener: (link: string) => void): void;
	/** true: no deck on this machine takes `scutoons-deck://` */
	deckMissing(): boolean;
	openDeckDownload(): void;
}

/** the installed creator's bridge; the web creator has none */
export function desktopBridge(scope: object = globalThis): DesktopBridge | null {
	const bridge = (scope as { scutoonsCreatorApp?: DesktopBridge }).scutoonsCreatorApp;
	return bridge && typeof bridge.onLink === 'function' && typeof bridge.deckMissing === 'function' ? bridge : null;
}

/**
 * Where a creator link leads: its item's page, else the workspace's own entry (`entryOf`: its last item or its start,
 * Home when it has neither). Not a creator link, a workspace the creator lacks, an item without a page: null.
 */
export function creatorLinkHref(link: string, entryOf: (kind: WorkspaceKind) => string | null): string | null {
	const read = readAppLink(link);
	if (!read || read.app !== CREATOR_APP || !ROW.includes(read.workspace as WorkspaceKind)) return null;
	if (read.address) return hrefOf(read.address);
	return entryOf(read.workspace as WorkspaceKind) ?? '/';
}
