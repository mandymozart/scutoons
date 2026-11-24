// The installers (`npm run dist`: Windows here, unsigned). macOS (dmg) and Arch Linux (pacman) are configured and untested;
// signing is the owner's (README.md). The scheme comes from the one link definition, server/schema/tables/deck-link.json.
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import type { Configuration } from 'electron-builder';
import { readSchemes } from './src/link.ts';

const here = import.meta.dirname ?? __dirname;
const NAME = 'SCUTOONS Creator';

/** Windows: the installer writes the scheme for the user (Electron writes the same key on start) and removes it on uninstall */
export function installerScript(scheme: string): string {
	const key = `Software\\Classes\\${scheme}`;
	return [
		'!macro customInstall',
		`  WriteRegStr HKCU "${key}" "" "URL:${NAME}"`,
		`  WriteRegStr HKCU "${key}" "URL Protocol" ""`,
		`  WriteRegStr HKCU "${key}\\DefaultIcon" "" "$INSTDIR\\\${APP_EXECUTABLE_FILENAME},0"`,
		`  WriteRegStr HKCU "${key}\\shell\\open\\command" "" '"$INSTDIR\\\${APP_EXECUTABLE_FILENAME}" "%1"'`,
		'!macroend',
		'!macro customUnInstall',
		`  DeleteRegKey HKCU "${key}"`,
		'!macroend',
		''
	].join('\r\n');
}

export function builderConfig(scheme: string): Configuration {
	return {
		appId: 'com.scutoons.creator',
		productName: NAME,
		executableName: 'scutoons-creator',
		directories: { output: 'dist', buildResources: 'resources' },
		files: ['out/*.js', 'package.json'],
		extraResources: [
			{ from: '../server/schema/tables/deck-link.json', to: 'deck-link.json' },
			{ from: 'config.json', to: 'config.json' },
			{ from: '../creator/static/app-icon-512.png', to: 'icon.png' }
		],
		icon: '../creator/static/app-icon-512.png',
		// the installed Electron of node_modules, no second download
		electronDist: 'node_modules/electron/dist',
		// macOS: Info.plist CFBundleURLTypes · Linux: the .desktop file's MimeType=x-scheme-handler/<scheme>
		protocols: [{ name: NAME, schemes: [scheme] }],
		win: { target: ['nsis'] },
		nsis: { oneClick: true, perMachine: false, include: 'out/installer.nsh' },
		mac: { target: ['dmg'], category: 'public.app-category.graphics-design' },
		linux: { target: ['pacman'], category: 'Graphics', synopsis: 'Write and publish SCUTOONS toons' }
	};
}

const schemes = readSchemes(JSON.parse(readFileSync(path.join(here, '../server/schema/tables/deck-link.json'), 'utf8')));
if (process.env.SCUTOONS_BUILDER_DRY !== '1') {
	// the downloads electron-builder needs (NSIS) stay in this folder, not in the user's profile
	process.env.ELECTRON_BUILDER_CACHE ??= path.join(here, '.cache', 'electron-builder');
	// an empty PATHEXT entry (this machine's has one) makes electron-builder find the extensionless `npm` shell script, not npm.cmd
	if (process.env.PATHEXT) process.env.PATHEXT = process.env.PATHEXT.split(';').filter(Boolean).join(';');
	mkdirSync(path.join(here, 'out'), { recursive: true });
	writeFileSync(path.join(here, 'out', 'installer.nsh'), installerScript(schemes.creator));
}

export default builderConfig(schemes.creator);
