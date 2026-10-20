// The update notice (issue 76, agents/MCP.md §3a): a member's copy (the one-file bundle / the Claude Desktop extension)
// asks GET /api/mcp/version once on start – non-blocking, a short timeout, silent on any error – and, when the server
// ships a newer one, whoami and the first other tool result carry one line with the download link. No auto-update.
// A checkout (`npm run mcp`) runs the sources and never checks.
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import { BUNDLED, type Config } from './config.ts';

const TIMEOUT_MS = 3000;
/** the member-facing short links (viewer/static/.htaccess → GET /api/mcp/download) */
const LINK = 'https://scutoons.com/mcp';
const LINK_MCPB = 'https://scutoons.com/mcp.mcpb';

let note: string | null = null;
let shown = false;

/** -1 / 0 / 1 for dotted numeric versions ("0.10.0" > "0.9.1") */
export function compareVersions(a: string, b: string): number {
	const pa = a.split('.').map((n) => Number.parseInt(n, 10) || 0);
	const pb = b.split('.').map((n) => Number.parseInt(n, 10) || 0);
	for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
		const d = (pa[i] ?? 0) - (pb[i] ?? 0);
		if (d !== 0) return Math.sign(d);
	}
	return 0;
}

/** newer = a higher version, or the same version with other bytes (a rebuild without a version bump) */
export function isNewer(remote: { version?: unknown; sha256?: unknown }, local: { version: string; sha256: string | null }): boolean {
	if (typeof remote.version !== 'string') return false;
	const order = compareVersions(remote.version, local.version);
	if (order !== 0) return order > 0;
	return typeof remote.sha256 === 'string' && remote.sha256 !== '' && local.sha256 !== null && remote.sha256 !== local.sha256;
}

export function updateLine(extension: boolean): string {
	return `Eine neue SCUTOONS-MCP-Version ist da: ${extension ? LINK_MCPB : LINK}`;
}

/** start the check (never awaited by the caller; never throws) */
export function checkForUpdate(config: Config, version: string): void {
	if (!BUNDLED) return;
	const file = fileURLToPath(import.meta.url);
	// the extension runs the same code as server/index.mjs
	const extension = basename(file) === 'index.mjs';
	let sha256: string | null = null;
	try {
		sha256 = createHash('sha256').update(readFileSync(file)).digest('hex');
	} catch {
		// unreadable – compare by version only
	}
	fetch(`${config.api}/mcp/version`, { signal: AbortSignal.timeout(TIMEOUT_MS) })
		.then((r) => (r.ok ? r.json() : null))
		.then((remote) => {
			if (remote && isNewer(remote, { version, sha256 })) note = updateLine(extension);
		})
		.catch(() => {});
}

/** the line for whoami (every call while an update is waiting) */
export function pendingUpdate(): string | null {
	return note;
}

/** the line for the first other tool result – once */
export function takeUpdateOnce(): string | null {
	if (!note || shown) return null;
	shown = true;
	return note;
}
