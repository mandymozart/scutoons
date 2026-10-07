// A throw-away Chrome profile for the headless checks (~120 MB each). Removed when the process exits – whatever path it
// takes (normal end, process.exit, a throw, Ctrl+C) – and stale ones of earlier runs that died are swept on start.
// Usage: `const profile = tempProfile('creator-dock-');` then `--user-data-dir=${profile}`; kill Chrome as before.
import { mkdtempSync, readdirSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const remove = (dir) => { try { rmSync(dir, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {} };

/** Makes `<tmp>/<prefix>XXXXXX`, removes it on exit; first sweeps `<prefix>*` folders older than two hours. */
export function tempProfile(prefix) {
	const root = tmpdir();
	const old = Date.now() - 2 * 3600_000;
	for (const name of readdirSync(root)) {
		if (!name.startsWith(prefix) || name.length !== prefix.length + 6) continue;
		try { if (statSync(join(root, name)).mtimeMs < old) remove(join(root, name)); } catch {}
	}
	const dir = mkdtempSync(join(root, prefix));
	// Chrome may still hold its files a moment after kill(): rmSync retries (EBUSY) for up to 2 s
	process.on('exit', () => remove(dir));
	process.once('SIGINT', () => process.exit(130));
	return dir;
}
