// The installed creator (#860): one window on the web creator – the same code, the same sign-in – that takes
// `scutoons-creator://` links, hands `scutoons-deck://` to the system and tells the page whether a deck is installed.
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { app, BrowserWindow, ipcMain, Menu, nativeTheme, session, shell, type IpcMainEvent, type WebContents } from 'electron';
import { deckInstalled } from './deck.ts';
import { frameOf, frameOptions, windowAction, type Frame } from './frame.ts';
import { goesOutside, linkIn, readConfig, readSchemes, sameOrigin } from './link.ts';

const root = path.join(__dirname, '..');
/** packaged: the copies electron-builder puts into resources/; from the checkout: the repo's own files */
const file = (packaged: string, checkout: string) => (app.isPackaged ? path.join(process.resourcesPath, packaged) : path.join(root, checkout));
const readJson = (at: string): unknown => JSON.parse(readFileSync(at, 'utf8'));

const schemes = readSchemes(readJson(file('deck-link.json', '../server/schema/tables/deck-link.json')));
const config = readConfig(readJson(file('config.json', 'config.json')), process.env);
const icon = file('icon.png', '../creator/static/app-icon-512.png');

let window: BrowserWindow | null = null;
/** links that came before the page could take them */
const pending: string[] = [];
let pageReady = false;
let deckMissing = false;

function deliver(link: string): void {
	if (window && pageReady) window.webContents.send('creator-app:link', link);
	else pending.push(link);
	focus();
}

function focus(): void {
	if (!window) return;
	if (window.isMinimized()) window.restore();
	window.show();
	window.focus();
}

const refreshDeck = async () => {
	deckMissing = !(await deckInstalled(schemes.deck));
};

/** the scheme for this app: the installer writes it too (Windows: installer.nsh, macOS: Info.plist, Linux: the .desktop file) */
function registerScheme(): void {
	if (!app.isPackaged) {
		// from the checkout only on request, so a test start does not take the links from an installed creator
		if (process.env.SCUTOONS_REGISTER_SCHEME === '1') app.setAsDefaultProtocolClient(schemes.creator, process.execPath, [path.resolve(process.argv[1] ?? '.')]);
		return;
	}
	app.setAsDefaultProtocolClient(schemes.creator);
}

/** only the creator's own page may ask the main process */
const fromCreator = (event: IpcMainEvent) => sameOrigin(event.senderFrame?.url ?? '', config.creatorUrl);

function openOutside(url: string): void {
	if (goesOutside(url, schemes)) void shell.openExternal(url);
}

function guard(contents: WebContents): void {
	contents.on('will-navigate', (event, url) => {
		if (sameOrigin(url, config.creatorUrl)) return;
		event.preventDefault();
		openOutside(url);
	});
	contents.setWindowOpenHandler(({ url }) => {
		if (sameOrigin(url, config.creatorUrl)) return { action: 'allow', overrideBrowserWindowOptions: { autoHideMenuBar: true, icon } };
		openOutside(url);
		return { action: 'deny' };
	});
}

function createWindow(): void {
	window = new BrowserWindow({
		width: 1440,
		height: 900,
		minWidth: 744,
		minHeight: 540,
		title: 'SCUTOONS Creator',
		icon,
		show: false,
		autoHideMenuBar: true,
		backgroundColor: nativeTheme.shouldUseDarkColors ? '#000000' : '#ffffff',
		...frameOptions(process.platform),
		webPreferences: { preload: path.join(__dirname, 'preload.js'), contextIsolation: true, sandbox: true, nodeIntegration: false, spellcheck: false }
	});
	if (process.platform !== 'darwin') window.removeMenu();
	window.once('ready-to-show', () => window?.show());
	const maximized = (on: boolean) => () => window?.webContents.send('creator-app:maximized', on);
	window.on('maximize', maximized(true));
	window.on('unmaximize', maximized(false));
	window.on('focus', () => void refreshDeck());
	window.on('closed', () => (window = null));
	window.webContents.on('did-start-navigation', (details) => {
		if (details.isMainFrame && !details.isSameDocument) pageReady = false;
	});
	void window.loadURL(config.creatorUrl);
}

/** macOS needs its menu for the app's name, copy and paste, and the window; elsewhere the window has none */
function menu(): void {
	if (process.platform !== 'darwin') return Menu.setApplicationMenu(null);
	Menu.setApplicationMenu(Menu.buildFromTemplate([{ role: 'appMenu' }, { role: 'editMenu' }, { role: 'windowMenu' }]));
}

if (!app.requestSingleInstanceLock()) {
	app.quit();
} else {
	const first = linkIn(process.argv, schemes.creator);
	if (first) pending.push(first);
	app.on('second-instance', (_event, argv) => {
		const link = linkIn(argv, schemes.creator);
		if (link) deliver(link);
		else focus();
	});
	// macOS hands a link as an event, also the one that started the app
	app.on('open-url', (event, url) => {
		event.preventDefault();
		deliver(url);
	});
	app.on('web-contents-created', (_event, contents) => guard(contents));

	ipcMain.on('creator-app:ready', (event) => {
		if (!fromCreator(event) || event.sender !== window?.webContents) return;
		pageReady = true;
		for (const link of pending.splice(0)) event.sender.send('creator-app:link', link);
	});
	ipcMain.on('creator-app:info', (event) => {
		event.returnValue = fromCreator(event) ? { deckDownload: config.deckDownloadUrl !== null } : null;
	});
	ipcMain.on('creator-app:deck-missing', (event) => {
		event.returnValue = fromCreator(event) && deckMissing;
	});
	ipcMain.on('creator-app:deck-download', (event) => {
		if (fromCreator(event) && config.deckDownloadUrl) void shell.openExternal(config.deckDownloadUrl);
	});
	// the main window has no system title bar (frame.ts); a window the page opened keeps the system's
	ipcMain.on('creator-app:frame', (event) => {
		const own = fromCreator(event) && event.sender === window?.webContents;
		const frame: Frame = own ? frameOf(process.platform) : 'system';
		event.returnValue = fromCreator(event) ? { frame, maximized: own && !!window?.isMaximized() } : null;
	});
	ipcMain.on('creator-app:window', (event, raw: unknown) => {
		const action = windowAction(raw);
		if (!action || !fromCreator(event) || !window || event.sender !== window.webContents) return;
		if (action === 'minimize') window.minimize();
		else if (action === 'close') window.close();
		else if (window.isMaximized()) window.unmaximize();
		else window.maximize();
	});

	void app.whenReady().then(async () => {
		registerScheme();
		menu();
		const creator = new URL(config.creatorUrl).origin;
		// the creator's own page may ask for what it uses (microphone, camera, MIDI …); nothing else may
		session.defaultSession.setPermissionRequestHandler((_contents, permission, callback, details) => {
			if (permission === 'openExternal') return callback(goesOutside('externalURL' in details ? (details.externalURL ?? '') : '', schemes));
			callback(sameOrigin(details.requestingUrl, creator));
		});
		session.defaultSession.setPermissionCheckHandler((_contents, _permission, origin) => sameOrigin(origin, creator));
		await refreshDeck();
		createWindow();
	});
	app.on('window-all-closed', () => app.quit());
}
