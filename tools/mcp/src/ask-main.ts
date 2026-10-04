// Starts the Ask service: `node bin/scutoons-ask.mjs` (README.md). Env: ANTHROPIC_API_KEY (locally also read from the
// repo-root .env), SCUTOONS_API (default: the local API), ASK_PORT (default shared/dev-ports.json "ask"), ASK_MODEL
// (default claude-sonnet-5-5), ASK_ORIGINS (comma list; local origins are always allowed against a local API),
// ASK_USAGE_LOG (default logs/ask/usage.jsonl), SCUTOONS_MCP_PUBLISH=1 (the publish tool, off by default).
import { existsSync, readFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { resolve } from 'node:path';
import Anthropic from '@anthropic-ai/sdk';
import { ROOT } from './config.ts';
import { handle, type AskConfig } from './ask.ts';

/** one key of the repo-root .env (the local secrets file, gitignored) */
function dotenv(key: string): string | undefined {
	const file = resolve(ROOT, '.env');
	if (!existsSync(file)) return undefined;
	const line = readFileSync(file, 'utf8').split(/\r?\n/).find((l) => l.startsWith(`${key}=`));
	return line?.slice(key.length + 1).trim().replace(/^["']|["']$/g, '') || undefined;
}

const ports = JSON.parse(readFileSync(resolve(ROOT, 'shared/dev-ports.json'), 'utf8'));
const apiKey = process.env.ANTHROPIC_API_KEY || dotenv('ANTHROPIC_API_KEY');
if (!apiKey) {
	console.error('scutoons-ask: no ANTHROPIC_API_KEY (env or the repo-root .env)');
	process.exit(1);
}
const config: AskConfig = {
	api: (process.env.SCUTOONS_API || `http://localhost:${ports.api}/api`).replace(/\/+$/, ''),
	model: process.env.ASK_MODEL || 'claude-sonnet-5-5',
	origins: (process.env.ASK_ORIGINS || 'https://creator.scutoons.com').split(',').map((o) => o.trim()).filter(Boolean),
	usageLog: process.env.ASK_USAGE_LOG || resolve(ROOT, 'logs/ask/usage.jsonl'),
	publish: process.env.SCUTOONS_MCP_PUBLISH === '1',
	anthropic: new Anthropic({ apiKey })
};
const port = Number(process.env.ASK_PORT || ports.ask);
createServer((req, res) => {
	handle(config, req, res).catch((error) => {
		console.error('scutoons-ask:', error);
		if (!res.headersSent) res.writeHead(500).end();
		else res.end();
	});
}).listen(port, () => console.error(`scutoons-ask: http://localhost:${port}/api/ask → ${config.api} (${config.model}, publish ${config.publish ? 'ON' : 'off'})`));
