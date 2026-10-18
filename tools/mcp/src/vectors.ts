// The vector library into the asset manager as the signed-in member (agents/research/VECTORS.md §6 step 5; owner
// 2026-10-18: "import the 436 vectors to production. Add an MCP tool that writes the shapes as my account"): the same
// core as the local script (scripts/vectors-core.mjs – svgo, `catpope/vectors/<slug>.svg`, title, tags, shape row),
// over this server's API with the member's token. Never replaces or deletes a file: a path held by an asset that is no
// shape stops the run before anything is written; a shape with other bytes is reported, not overwritten. Developers
// who write content only; dry run by default, then in batches (`limit`) – a second run changes nothing.
import type { McpServer } from '@modelcontextprotocol/server';
import * as z from 'zod';
import { createApiClient, createAssetsApi } from '@scutoons/api';
import { importVectors } from '../../../scripts/vectors-core.mjs';
import type { Config } from './config.ts';
import { apiMessage } from './server.ts';

type Result = { content: { type: 'text'; text: string }[]; isError?: boolean };
const text = (value: unknown): Result => ({ content: [{ type: 'text', text: typeof value === 'string' ? value : JSON.stringify(value, null, 1) }] });

export function registerVectorTools(server: McpServer, config: Config): void {
	const assets = createAssetsApi(createApiClient({ baseUrl: config.api, getToken: () => config.token }));

	server.registerTool(
		'import_vectors',
		{
			description:
				'Imports the vector library (shared/tokens/vectors/vectors.json in this checkout, agents/research/VECTORS.md) into the asset manager of the API this server talks to, as you: each SVG shrunk (svgo) to catpope/vectors/<slug>.svg with title, tags and its shape row (GET /api/assets?kind=shape). `dry_run` (default true) only plans: counts of upload / update / unchanged, conflicts (a shape with other bytes – never overwritten) and collisions (the path held by something that is no shape – then nothing is written). A real run writes at most `limit` shapes per call; `left` says how many remain – call again. Idempotent.',
			inputSchema: z.object({
				dry_run: z.boolean().default(true),
				limit: z.number().int().min(1).max(500).default(100).describe('at most this many shapes written per call'),
				slugs: z.array(z.string()).optional().describe('only these entries')
			}),
			annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: true, openWorldHint: true }
		},
		async ({ dry_run, limit, slugs }) => {
			const started = Date.now();
			try {
				const result = await importVectors({
					dryRun: dry_run,
					limit,
					only: new Set(slugs ?? []),
					async find(key) {
						const { asset } = await assets.lookup(key);
						if (!asset) return null;
						const current = asset.versions.find((v) => v.status === 'current');
						return { id: asset.asset.id, sha256: current?.sha256 ?? null, title: asset.asset.title, tags: asset.asset.tags, shape: asset.asset.shape ?? null };
					},
					upload: async (bytes, key, title, tags) => (await assets.upload(new Blob([new Uint8Array(bytes)], { type: 'image/svg+xml' }), key, { title, tags })).asset.id,
					save: async (id, body) => void (await assets.update(id, body))
				});
				return { ...text({ api: config.api, dry_run, ...result, seconds: Math.round((Date.now() - started) / 1000) }), isError: result.collision.length > 0 || result.failed.length > 0 };
			} catch (error) {
				return { ...text(apiMessage(error)), isError: true };
			}
		}
	);
}
