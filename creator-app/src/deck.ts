// Whether this machine has a deck to take `scutoons-deck://`: its installer registers the scheme (DECK-LINK.md).
import { execFile } from 'node:child_process';
import { app } from 'electron';

/** Windows: the scheme's open command in the registry (HKCR = the user's and the machine's classes) */
const inRegistry = (scheme: string): Promise<boolean> =>
	new Promise((resolve) => {
		execFile('reg', ['query', `HKCR\\${scheme}\\shell\\open\\command`, '/ve'], { windowsHide: true, timeout: 5000 }, (error) => resolve(!error));
	});

/** macOS (Launch Services) and Linux (xdg-mime): the application the system names for the scheme */
const hasHandler = (scheme: string): boolean => app.getApplicationNameForProtocol(`${scheme}://`) !== '';

export async function deckInstalled(scheme: string): Promise<boolean> {
	try {
		return process.platform === 'win32' ? await inRegistry(scheme) : hasHandler(scheme);
	} catch {
		return true; // unknown: the link is tried, as in the web creator
	}
}
