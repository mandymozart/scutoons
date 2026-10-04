// The admin's own tools for the Ask panel in the admin app (role admin, checked by the API): read-only to start –
// next to the content tools that only read (ask.ts) – the AI usage overview. Catalog tools come later (CUTOVER 10.28).
import { McpServer } from '@modelcontextprotocol/server';
import * as z from 'zod';
import { createApiClient } from '@scutoons/api';
import type { Config } from './config.ts';
import { apiMessage } from './server.ts';

type Result = { content: { type: 'text'; text: string }[]; isError?: boolean };
const text = (value: unknown): Result => ({ content: [{ type: 'text', text: typeof value === 'string' ? value : JSON.stringify(value, null, 1) }] });

export function createAdminServer(config: Config): McpServer {
	const client = createApiClient({ baseUrl: config.api, getToken: () => config.token });
	const server = new McpServer({ name: 'scutoons-admin', version: '0.1.0' }, { instructions: 'SCUTOONS admin: read-only overviews for the signed-in admin.' });

	server.registerTool(
		'agent_usage',
		{
			description: 'What the AI agents cost: calls, tokens and USD per user, app (creator / embassy / admin), service and model, per day or month. Dates YYYY-MM-DD (default: this month).',
			inputSchema: z.object({
				from: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).optional(),
				to: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).optional(),
				group: z.enum(['day', 'month']).optional()
			}),
			annotations: { readOnlyHint: true, openWorldHint: false }
		},
		async ({ from, to, group }) => {
			try {
				const query = new URLSearchParams(Object.entries({ from, to, group }).filter((e): e is [string, string] => !!e[1])).toString();
				return text(await client.get(`/admin/agent-usage${query ? `?${query}` : ''}`));
			} catch (error) {
				return { ...text(apiMessage(error)), isError: true };
			}
		}
	);

	return server;
}
