// Where the server talks to and as whom: `SCUTOONS_API` (default: the local dev API) + `SCUTOONS_TOKEN` (a creator's
// session token, sent as `Authorization: Bearer`). Locally the token falls back to the test user's session
// (`npm run dev:test-user` → logs/dev/test-user.json) – never for a remote API. No publishing (agents/MCP.md: the
// member publishes in the creator).
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/** the repository root (tools/mcp/src → ../../..) */
export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');

const LOCAL_HOSTS = new Set(['localhost', '127.0.0.1', '[::1]', '::1']);

export interface Config {
	api: string;
	token: string;
	/** where the token came from (shown in the server's instructions, never the token) */
	tokenSource: 'env' | 'test-user';
	local: boolean;
}

function localApi(): string {
	const ports = JSON.parse(readFileSync(resolve(ROOT, 'shared/dev-ports.json'), 'utf8'));
	return `http://localhost:${ports.api}/api`;
}

export function isLocalUrl(url: string): boolean {
	try {
		return LOCAL_HOSTS.has(new URL(url).hostname);
	} catch {
		return false;
	}
}

export function loadConfig(env: NodeJS.ProcessEnv = process.env): Config {
	const api = (env.SCUTOONS_API || localApi()).replace(/\/+$/, '');
	const local = isLocalUrl(api);
	let token = env.SCUTOONS_TOKEN?.trim() || '';
	let tokenSource: Config['tokenSource'] = 'env';
	if (!token && local) {
		const file = resolve(ROOT, 'logs/dev/test-user.json');
		if (existsSync(file)) {
			token = String(JSON.parse(readFileSync(file, 'utf8')).token ?? '');
			tokenSource = 'test-user';
		}
	}
	if (!token) {
		throw new Error(
			local
				? 'no token: set SCUTOONS_TOKEN or run `npm run dev:test-user` (writes logs/dev/test-user.json)'
				: `no token for ${api}: set SCUTOONS_TOKEN to your personal token (scutoons.com/account → AI connections)`
		);
	}
	return { api, token, tokenSource, local };
}
