// stdio entry: `node bin/scutoons-mcp.mjs` (Claude Code / Claude Desktop start it; README.md). Logs go to stderr only –
// stdout is the MCP channel.
import { StdioServerTransport } from '@modelcontextprotocol/server/stdio';
import { loadConfig } from './config.ts';
import { createScutoonsServer } from './server.ts';

try {
	const config = loadConfig();
	const server = createScutoonsServer(config);
	await server.connect(new StdioServerTransport());
	console.error(`scutoons-mcp: ${config.api} (token: ${config.tokenSource})`);
} catch (error) {
	console.error(`scutoons-mcp: ${error instanceof Error ? error.message : error}`);
	process.exit(1);
}
