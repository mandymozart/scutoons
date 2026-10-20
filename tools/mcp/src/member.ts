// The SCUTOONS MCP server for one member (agents/MCP.md): their roles pick the services – content (creator; an admin
// reads), Embassy (liaison / ambassador / admin), admin – and only those services' tools, prompts and rules are listed.
// The API stays the only authority: every call carries the member's token.
import { McpServer } from '@modelcontextprotocol/server';
import * as z from 'zod';
import { registerAdminTools } from './admin.ts';
import type { Config } from './config.ts';
import { registerDevTools } from './dev.ts';
import { registerEmbassyTools } from './embassy.ts';
import { registerFigmaTools } from './figma.ts';
import { instructions, registerPrompts } from './prompts.ts';
import { registerContentTools } from './server.ts';
import { appLinks, type Member, type Services } from './services.ts';
import { registerStoryTools } from './story.ts';
import { pendingUpdate, takeUpdateOnce } from './update.ts';
import { registerVectorTools } from './vectors.ts';

/** = tools/mcp/package.json version (bump both; GET /api/mcp/version compares it – update.ts) */
export const VERSION = '0.4.0';

export function createMemberServer(config: Config, member: Member, services: Services): McpServer {
	const server = new McpServer({ name: 'scutoons', version: VERSION }, { instructions: instructions(services, config.api, config.local) });
	const links = appLinks(config);
	withUpdateNote(server);

	server.registerTool(
		'whoami',
		{
			description: 'Who you work for (name, email, roles), which SCUTOONS services that opens, and the app links drafts are confirmed in. Call it first.',
			inputSchema: z.object({}),
			annotations: { readOnlyHint: true, openWorldHint: false }
		},
		async () => ({
			content: [
				{
					type: 'text',
					text: JSON.stringify(
						{
							member: { name: member.name || null, email: member.email, roles: member.roles },
							services: {
								content: services.content ?? 'none',
								embassy: services.embassy,
								admin: services.admin,
								developer: services.developer
							},
							apps: links,
							api: config.local ? `${config.api} (local dev)` : config.api,
							confirm: 'Saves are drafts. Give the member the link from each result: they publish (creator), send (Embassy) or apply (admin proposals) in the browser.',
							...(pendingUpdate() ? { update: `${pendingUpdate()} – tell the member (download, replace the old file, restart Claude).` } : {})
						},
						null,
						1
					)
				}
			]
		})
	);

	if (services.content) {
		registerContentTools(server, config, { write: services.content === 'write', links });
		registerStoryTools(server, config);
		// the member's own Figma file through the SCUTOONS Figma bridge (tools/figma-bridge) – creators who write
		if (services.content === 'write') registerFigmaTools(server);
	}
	if (services.embassy) registerEmbassyTools(server, config, { member, links });
	if (services.admin) registerAdminTools(server, config, { links });
	if (services.developer) registerDevTools(server, config);
	// the vector library into the asset manager (research/VECTORS.md) – developers who also write content
	if (services.developer && services.content === 'write') registerVectorTools(server, config);
	registerPrompts(server, services);
	return server;
}

/** every tool but whoami (which has its own field) carries the update line once – update.ts */
function withUpdateNote(server: McpServer): void {
	const register = server.registerTool.bind(server) as (...args: unknown[]) => unknown;
	type Handler = (...args: unknown[]) => unknown;
	(server as unknown as { registerTool: (...args: unknown[]) => unknown }).registerTool = (name: unknown, spec: unknown, handler: unknown) => {
		if (name === 'whoami' || typeof handler !== 'function') return register(name, spec, handler);
		const wrapped: Handler = async (...args) => {
			const result = (await (handler as Handler)(...args)) as { content?: unknown[] } | undefined;
			const line = result && Array.isArray(result.content) ? takeUpdateOnce() : null;
			if (line && result?.content) result.content.push({ type: 'text', text: `${line} – tell the member.` });
			return result;
		};
		return register(name, spec, wrapped);
	};
}
