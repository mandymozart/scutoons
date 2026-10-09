// The SCUTOONS MCP server for one member (agents/MCP.md): their roles pick the services – content (creator; an admin
// reads), Embassy (liaison / ambassador / admin), admin – and only those services' tools, prompts and rules are listed.
// The API stays the only authority: every call carries the member's token.
import { McpServer } from '@modelcontextprotocol/server';
import * as z from 'zod';
import { registerAdminTools } from './admin.ts';
import type { Config } from './config.ts';
import { registerEmbassyTools } from './embassy.ts';
import { registerFigmaTools } from './figma.ts';
import { instructions, registerPrompts } from './prompts.ts';
import { registerContentTools } from './server.ts';
import { appLinks, type Member, type Services } from './services.ts';
import { registerStoryTools } from './story.ts';

export const VERSION = '0.3.0';

export function createMemberServer(config: Config, member: Member, services: Services): McpServer {
	const server = new McpServer({ name: 'scutoons', version: VERSION }, { instructions: instructions(services, config.api, config.local) });
	const links = appLinks(config);

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
								admin: services.admin
							},
							apps: links,
							api: config.local ? `${config.api} (local dev)` : config.api,
							confirm: 'Saves are drafts. Give the member the link from each result: they publish (creator), send (Embassy) or apply (admin proposals) in the browser.'
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
	registerPrompts(server, services);
	return server;
}
