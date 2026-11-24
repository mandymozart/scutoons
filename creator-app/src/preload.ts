// The bridge the creator's page sees as `window.scutoonsCreatorApp` (creator/src/lib/shell/desktop.ts): links handed to the
// app, whether a deck is installed, and the deck's download. The main process answers only the creator's own origin.
import { contextBridge, ipcRenderer } from 'electron';

const info = (ipcRenderer.sendSync('creator-app:info') as { deckDownload: boolean } | null) ?? { deckDownload: false };
const early: string[] = [];
let listener: ((link: string) => void) | null = null;

ipcRenderer.on('creator-app:link', (_event, link: string) => {
	if (listener) listener(link);
	else early.push(link);
});
ipcRenderer.send('creator-app:ready');

contextBridge.exposeInMainWorld('scutoonsCreatorApp', {
	hasDeckDownload: info.deckDownload,
	onLink(next: (link: string) => void): void {
		listener = next;
		for (const link of early.splice(0)) next(link);
	},
	deckMissing: (): boolean => ipcRenderer.sendSync('creator-app:deck-missing') === true,
	openDeckDownload: (): void => ipcRenderer.send('creator-app:deck-download')
});
