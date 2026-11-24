// The bridge the creator's page sees as `window.scutoonsCreatorApp` (creator/src/lib/shell/desktop.ts): links handed to the
// app, whether a deck is installed, the deck's download, and the window's own buttons (minimize, maximize / restore, close
// and the maximized state). The main process answers only the creator's own origin.
import { contextBridge, ipcRenderer } from 'electron';

const info = (ipcRenderer.sendSync('creator-app:info') as { deckDownload: boolean } | null) ?? { deckDownload: false };
const early: string[] = [];
let listener: ((link: string) => void) | null = null;

ipcRenderer.on('creator-app:link', (_event, link: string) => {
	if (listener) listener(link);
	else early.push(link);
});
ipcRenderer.send('creator-app:ready');

type Frame = 'own' | 'inset' | 'system';
const frame = (ipcRenderer.sendSync('creator-app:frame') as { frame: Frame; maximized: boolean } | null) ?? { frame: 'system' as Frame, maximized: false };
let maximized = frame.maximized;
let onMaximized: ((on: boolean) => void) | null = null;
ipcRenderer.on('creator-app:maximized', (_event, on: boolean) => {
	maximized = on === true;
	onMaximized?.(maximized);
});
const act = (action: 'minimize' | 'maximize' | 'close') => () => ipcRenderer.send('creator-app:window', action);

contextBridge.exposeInMainWorld('scutoonsCreatorApp', {
	/** who draws the window buttons: `own` the page, `inset` macOS, `system` a framed window */
	frame: frame.frame,
	/** the window's maximized state now, then every change */
	onMaximized(next: (on: boolean) => void): void {
		onMaximized = next;
		next(maximized);
	},
	minimize: act('minimize'),
	toggleMaximize: act('maximize'),
	close: act('close'),
	hasDeckDownload: info.deckDownload,
	onLink(next: (link: string) => void): void {
		listener = next;
		for (const link of early.splice(0)) next(link);
	},
	deckMissing: (): boolean => ipcRenderer.sendSync('creator-app:deck-missing') === true,
	openDeckDownload: (): void => ipcRenderer.send('creator-app:deck-download')
});
