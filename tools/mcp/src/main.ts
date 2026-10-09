// stdio entry: `node bin/scutoons-mcp.mjs` (Claude Code / Claude Desktop start it; README.md). Logs go to stderr only –
// stdout is the MCP channel. Readers (no member role) can't connect (owner 2026-10-09).
import { StdioServerTransport } from '@modelcontextprotocol/server/stdio';
import { loadConfig } from './config.ts';
import { createMemberServer } from './member.ts';
import { fetchMember, hasAny, servicesOf } from './services.ts';

try {
	const config = loadConfig();
	const member = await fetchMember(config);
	if (!member) throw new Error(`the token doesn't work at ${config.api} – expired (six months) or revoked: renew it or create a new one on scutoons.com/account → AI connections`);
	const services = servicesOf(member.roles);
	if (!hasAny(services)) throw new Error(`${member.email} has no member role (creator, liaison, ambassador, admin) – SCUTOONS AI connections are for members only`);
	const server = createMemberServer(config, member, services);
	await server.connect(new StdioServerTransport());
	const open = [services.content && `content (${services.content})`, services.embassy && 'embassy', services.admin && 'admin'].filter(Boolean).join(', ');
	console.error(`scutoons-mcp: ${config.api} as ${member.email} (token: ${config.tokenSource}) – ${open}`);
} catch (error) {
	console.error(`scutoons-mcp: ${error instanceof Error ? error.message : error}`);
	process.exit(1);
}
