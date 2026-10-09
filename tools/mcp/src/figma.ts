// Figma through the SCUTOONS Figma bridge (tools/figma-bridge, agents/STORYBOARD.md §9): the member's own Figma file,
// open in Figma desktop with the "SCUTOONS Bridge" development plugin, reached through the relay on 127.0.0.1. No
// Figma API, no token, no monthly cap – the plugin acts as the member, only while its window is open. Read the
// selection, look at frames as pictures, place storyboard draft frames (scripts/storyboard/figma-draft.mjs specs).
import type { McpServer } from '@modelcontextprotocol/server';
import { readFileSync } from 'node:fs';
import * as z from 'zod';

type Content = { type: 'text'; text: string } | { type: 'image'; data: string; mimeType: string };
type Result = { content: Content[]; isError?: boolean };
const text = (value: unknown): Result => ({ content: [{ type: 'text', text: typeof value === 'string' ? value : JSON.stringify(value) }] });
const fail = (message: string): Result => ({ ...text(message), isError: true });

const SETUP =
	'The SCUTOONS Figma bridge is not ready: (1) `node tools/figma-bridge/relay.mjs` running on this computer, (2) in Figma desktop the file open and Plugins → Development → SCUTOONS Bridge started (green dot). Setup: tools/figma-bridge/README.md.';

export function figmaPort(): number {
	return Number(process.env.SCUTOONS_FIGMA_PORT ?? 3056);
}

/** one command through the relay → the plugin's answer ({ ok, result } / { ok: false, error }) */
export async function bridge(cmd: string, args: Record<string, unknown> = {}, port = figmaPort()): Promise<{ ok: boolean; result?: any; error?: string }> {
	let res: Response;
	try {
		res = await fetch(`http://127.0.0.1:${port}/cmd`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ cmd, args }) });
	} catch {
		return { ok: false, error: SETUP };
	}
	const out = (await res.json()) as { ok: boolean; result?: unknown; error?: string };
	if (!out.ok && /no plugin connected/.test(out.error ?? '')) return { ok: false, error: SETUP };
	return out;
}

const read = { readOnlyHint: true, openWorldHint: false };

export function registerFigmaTools(server: McpServer, opts: { port?: number } = {}) {
	const port = opts.port ?? figmaPort();
	const call = async (cmd: string, args: Record<string, unknown> = {}) => {
		const r = await bridge(cmd, args, port);
		if (!r.ok) throw new Error(r.error ?? 'bridge error');
		return r.result;
	};
	const guard =
		<A>(fn: (args: A) => Promise<Result>) =>
		async (args: A): Promise<Result> => {
			try {
				return await fn(args);
			} catch (error) {
				return fail(error instanceof Error ? error.message : String(error));
			}
		};

	server.registerTool(
		'figma_status',
		{
			description:
				'Is the SCUTOONS Figma bridge ready (relay running, plugin open in Figma desktop)? Returns the open file, page and user, and the file\'s pages. Call it first; every other figma_* tool needs it. The bridge has no call limit (unlike Figma\'s own MCP).',
			inputSchema: z.object({}),
			annotations: read
		},
		guard(async () => {
			const ping = await call('ping');
			const pages = await call('pages');
			return text({ ...ping, pages });
		})
	);

	server.registerTool(
		'figma_read',
		{
			description:
				'The node tree of a page or a node in the open Figma file: id, name, type, x / y / w / h, text of TEXT layers, image hash of image fills, locked / hidden. `page` = a page name or id (e.g. "S1C3", the chapter page with the owner\'s selection); `node` = a node id ("555:2354", from a Figma URL node-id=555-2354); neither = the current page. `depth` limits the levels (default 2) – go deeper on the node you need.',
			inputSchema: z.object({
				page: z.string().optional(),
				node: z.string().optional().describe('node id, "555:2354" (a URL\'s 555-2354 works too)'),
				depth: z.number().int().min(0).max(6).optional()
			}),
			annotations: read
		},
		guard(async ({ page, node, depth }) => text(await call('read', { page, nodeId: node?.replace('-', ':'), depth: depth ?? 2 })))
	);

	server.registerTool(
		'figma_view',
		{
			description:
				'Look at a node of the open Figma file as a picture (PNG): a photo of the selection, a frame, a layouted draft. `scale` 0.1–2 (default 0.5 – thumbnails are enough to match shots; use 1 for detail). Large frames at a small scale.',
			inputSchema: z.object({ node: z.string(), scale: z.number().min(0.05).max(2).optional() }),
			annotations: read
		},
		guard(async ({ node, scale }) => {
			const r = await call('export', { nodeId: node.replace('-', ':'), format: 'PNG', scale: scale ?? 0.5 });
			if (r.base64.length > 4_000_000) return fail(`the picture is ${Math.round(r.bytes / 1024)} KiB – take a smaller scale`);
			return { content: [{ type: 'text', text: `${r.name} (${Math.round(r.bytes / 1024)} KiB)` }, { type: 'image', data: r.base64, mimeType: 'image/png' }] };
		})
	);

	server.registerTool(
		'figma_place_draft',
		{
			description:
				'Places or updates a storyboard draft frame in the open Figma file from a spec made by scripts/storyboard/figma-draft.mjs (`storyboards/_out/<chapter>/figma/scene-<n>.spec.json`): on its own page, a 390-wide frame with the locked `storyboard` layer (schematic shots), hidden `guides`, `text` (the dialog as TEXT layers) and an empty `layout` layer on top, the rhythm rail beside it. A re-run replaces the shots by id and never touches `layout`. Writes into the member\'s Figma file – say which page and frames before you call it.',
			inputSchema: z.object({ spec_file: z.string().describe('local path of a scene-<n>.spec.json') }),
			annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: true, openWorldHint: false }
		},
		guard(async ({ spec_file }) => {
			const spec = JSON.parse(readFileSync(spec_file, 'utf8'));
			return text({ placed: spec.frame?.name, page: spec.page, ...(await call('draft', { spec })) });
		})
	);
}
