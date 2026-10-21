// Figma through the SCUTOONS Figma bridge (tools/figma-bridge, agents/STORYBOARD.md §9): the member's own Figma file,
// open in Figma desktop with the "SCUTOONS Bridge" development plugin, reached through the relay on 127.0.0.1. No
// Figma API, no token, no monthly cap – the plugin acts as the member, only while its window is open. Read the
// selection, look at frames as pictures, place storyboard draft frames (scripts/storyboard/figma-draft.mjs specs).
// The relay (#127): a running one is used (the owner's relay.mjs under supervise.mjs); with none on the port, a member's
// copy starts its own in-process (figma-relay.ts) – from a checkout only with SCUTOONS_FIGMA_RELAY=mcp (relayAutoStart).
import type { McpServer } from '@modelcontextprotocol/server';
import { readFileSync } from 'node:fs';
import * as z from 'zod';
import { BUNDLED } from './config.ts';
import { commandSource } from './figma-commands.ts';
import { relayAutoStart, type ModuleSource } from './figma-relay-core.ts';
import { ensureRelay } from './figma-relay.ts';

type Content = { type: 'text'; text: string } | { type: 'image'; data: string; mimeType: string };
type Result = { content: Content[]; isError?: boolean };
const text = (value: unknown): Result => ({ content: [{ type: 'text', text: typeof value === 'string' ? value : JSON.stringify(value) }] });
const fail = (message: string): Result => ({ ...text(message), isError: true });

const PLUGIN = 'in Figma desktop the file open and the SCUTOONS Bridge started (Ctrl+Alt+P "Run last plugin", or Plugins → Development → SCUTOONS Bridge; green dot)';
/** the plugin isn't installed yet? (a member's copy: the download beside the MCP) */
const INSTALL = 'Not installed yet: download https://scutoons.com/figma-plugin, unzip it, then Figma desktop → Plugins → Development → Import plugin from manifest… → its manifest.json';
const SETUP = `The SCUTOONS Figma bridge is not ready: (1) the relay running on this computer – \`node tools/figma-bridge/supervise.mjs --detach\` keeps tools/figma-bridge/relay.mjs running in the background, (2) ${PLUGIN}. Setup: tools/figma-bridge/README.md.`;
const noPlugin = (auto: boolean) => `The relay runs, but no plugin is connected: ${PLUGIN}.${auto ? ` ${INSTALL}.` : ''}`;

/**
 * per command (ms): how long the plugin may work on it (the relay answers `timeout` after that, Figma finishes it
 * anyway) – plus up to WAIT in the relay's queue behind another call (a script's export run). Together below fetch's
 * own 300 s limit (undici headersTimeout), so a slow call ends with a clear message, not "fetch failed".
 */
const TIMEOUTS: Record<string, number> = { ping: 15_000, pages: 30_000, read: 180_000, export: 120_000, draft: 180_000 };
const WAIT = 60_000;

export function figmaPort(): number {
	return Number(process.env.SCUTOONS_FIGMA_PORT ?? 3056);
}

/** the relay's error code → what the member (and their AI) should know */
function explain(cmd: string, code: string | undefined, error: string | undefined, timeout: number, auto: boolean): string {
	switch (code) {
		case 'no_plugin':
			return noPlugin(auto);
		case 'plugin_disconnected':
			return `The plugin went away during ${cmd} (Figma closed or reloaded it). ${noPlugin(auto)}`;
		case 'timeout':
			return `Figma did not answer ${cmd} within ${Math.round(timeout / 1000)} s – it is still working on it; the bridge sends the next call when it is done. Try again in a minute, read a node instead of a whole page, or view at a smaller scale.`;
		case 'queue_timeout':
			return `The bridge is busy with another call (a script's export run?) – ${cmd} was not sent. Try again in a few minutes.`;
		case 'forbidden':
			return `The relay refused the call (${error ?? 'forbidden'}) – it takes calls from local tools only.`;
		default:
			return error ?? `${cmd} failed`;
	}
}

export interface BridgeOptions {
	/** start this MCP's own relay when nothing listens (default: relayAutoStart – a member's copy, or SCUTOONS_FIGMA_RELAY=mcp) */
	autoStart?: boolean;
	/** the command modules for that relay (default: the checkout's tools/figma-bridge/commands, embedded in a member's copy) */
	modules?: ModuleSource;
	/** after starting the relay: how long to wait for an open plugin window to reconnect (ms, default 11 s) */
	pluginWait?: number;
}

const post = (port: number, body: string, ms: number) =>
	fetch(`http://127.0.0.1:${port}/cmd`, { method: 'POST', headers: { 'content-type': 'application/json' }, body, signal: AbortSignal.timeout(ms) });

/** one command through the relay → the plugin's answer ({ ok, result } / { ok: false, error }) */
export async function bridge(cmd: string, args: Record<string, unknown> = {}, port = figmaPort(), opts: BridgeOptions = {}): Promise<{ ok: boolean; result?: any; error?: string }> {
	const timeout = TIMEOUTS[cmd] ?? 120_000;
	const auto = opts.autoStart ?? relayAutoStart(process.env, BUNDLED);
	const body = JSON.stringify({ cmd, args, timeout, wait: WAIT, client: 'scutoons-mcp' });
	let res: Response;
	try {
		try {
			res = await post(port, body, timeout + WAIT + 20_000);
		} catch (error) {
			if (!auto || (error as { cause?: { code?: string } }).cause?.code !== 'ECONNREFUSED') throw error;
			// nothing on the port: this MCP's own relay (the plugin finds it within 10 s – ui.html reconnects by itself)
			try {
				await ensureRelay(port, opts.modules ?? commandSource);
			} catch (e) {
				return { ok: false, error: `The SCUTOONS Figma relay could not start on 127.0.0.1:${port} (${(e as Error).message}) – another program uses the port? Set SCUTOONS_FIGMA_PORT to a free one (the plugin needs 3056).` };
			}
			// an open plugin window retries every ≤ 10 s – give it that long before answering no_plugin
			for (let t0 = Date.now(); Date.now() - t0 < (opts.pluginWait ?? 11_000); ) {
				if ((await relayStatus(port))?.plugin) break;
				await new Promise((ok) => setTimeout(ok, 250));
			}
			res = await post(port, body, timeout + WAIT + 20_000);
		}
	} catch (error) {
		const e = error as { name?: string; cause?: { code?: string } };
		if (e.cause?.code === 'ECONNREFUSED') return { ok: false, error: SETUP };
		if (e.name === 'TimeoutError') return { ok: false, error: `The SCUTOONS Figma relay did not answer ${cmd} within ${Math.round((timeout + WAIT + 20_000) / 1000)} s (node tools/figma-bridge/cli.mjs status shows what it is doing).` };
		return { ok: false, error: `The connection to the SCUTOONS Figma relay broke (${e.cause?.code ?? e.name ?? 'unknown'}) – it may be restarting; try again in a moment.` };
	}
	let out: { ok: boolean; result?: unknown; code?: string; error?: string };
	try {
		out = await res.json();
	} catch {
		return { ok: false, error: `${SETUP} (port ${port} answers, but not as the relay – HTTP ${res.status})` };
	}
	if (out.ok) return out;
	// a relay from before #78 answers without a code
	const code = out.code ?? (/no plugin connected/.test(out.error ?? '') ? 'no_plugin' : undefined);
	return { ok: false, error: explain(cmd, code, out.error, timeout, auto) };
}

/** the relay's GET /status on the port, or null (nothing there / not a relay) */
export async function relayStatus(port = figmaPort(), ms = 1_500): Promise<Record<string, unknown> | null> {
	try {
		const res = await fetch(`http://127.0.0.1:${port}/status`, { signal: AbortSignal.timeout(ms) });
		const s = (await res.json()) as Record<string, unknown>;
		return s && typeof s === 'object' && 'plugin' in s ? s : null;
	} catch {
		return null;
	}
}

const read = { readOnlyHint: true, openWorldHint: false };

export function registerFigmaTools(server: McpServer, opts: { port?: number } & BridgeOptions = {}) {
	const port = opts.port ?? figmaPort();
	const call = async (cmd: string, args: Record<string, unknown> = {}) => {
		const r = await bridge(cmd, args, port, opts);
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
