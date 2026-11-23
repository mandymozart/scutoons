// XR experiences in the MCP (research/XR.md §10.8): a published OSCT content version becomes the draft of a catalog
// document `experience:<id>` – its files go to the asset manager under the experience's own folder, its compiled
// plugins into the document as code. A draft only: the member publishes it; the reader plays it at /xr/<id>.
import type { McpServer } from '@modelcontextprotocol/server';
import * as z from 'zod';
import { ApiError, createApiClient, createAssetsApi, createContentApi } from '@scutoons/api';
import { XR_EXPERIENCE_TYPES, type XrExperience, type XrGameConfiguration, type XrPlugin } from '@scutoons/content';
import type { Config } from './config.ts';
import { apiMessage, mimeOf } from './server.ts';
import type { AppLinks } from './services.ts';
import { planImport } from './xr-import.ts';

type Result = { content: { type: 'text'; text: string }[]; isError?: boolean };
const text = (value: unknown): Result => ({ content: [{ type: 'text', text: typeof value === 'string' ? value : JSON.stringify(value, null, 1) }] });
const fail = (message: string): Result => ({ ...text(message), isError: true });

/** the server's limit for one plugin's code (CatalogValidator) */
const PLUGIN_BYTES = 262_144;
const EXPERIENCE_ID = /^[a-z][a-z0-9-]{1,63}$/;

export function registerXrTools(server: McpServer, config: Config, options: { write: boolean; links: AppLinks; fetch?: typeof fetch }): void {
	if (!options.write) return;
	const get = options.fetch ?? fetch;
	const client = createApiClient({ baseUrl: config.api, getToken: () => config.token, fetch: options.fetch });
	const assets = createAssetsApi(client);
	const content = createContentApi(client);

	const download = async (url: string): Promise<Response> => {
		const res = await get(url);
		if (!res.ok) throw new Error(`download failed: ${url} → ${res.status} ${res.statusText}`);
		return res;
	};

	server.registerTool(
		'import_xr_experience',
		{
			description:
				'Takes a PUBLISHED OSCT content version as the draft of a SCUTOONS XR experience (`experience:<id>`, played by the reader at /xr/<id>). `source` = the content base of the OSCT experience (the folder that holds latest.json, e.g. "https://…/osct/staging/demo"); `build` = one published version instead of the latest (the OSCT editor names it in its publish log); `scene` = only this OSCT scene with its entries and the shared entities they use (without it the whole experience). Uploads the files the configuration uses to the asset manager under xr/<id>/<build>/ (a file already there is kept), puts the compiled plugins of the version\'s newest plugin API into the document as code, and saves a new draft version. `dry_run` only lists what it would take. Nothing is published: the member publishes the draft.',
			inputSchema: z.object({
				source: z.string().url().describe('the OSCT experience\'s content base URL'),
				scene: z.string().optional().describe('one OSCT scene id'),
				build: z.string().regex(/^[0-9a-f]{6,64}$/).optional().describe('one published version (its build); default: the one latest.json names'),
				id: z.string().regex(EXPERIENCE_ID).describe('the SCUTOONS experience id, e.g. poster-catpope'),
				type: z.enum(XR_EXPERIENCE_TYPES),
				title: z.record(z.string(), z.string()).describe('{ "<locale>": "…" }'),
				toons: z.array(z.string()).describe('the toons it belongs to'),
				dry_run: z.boolean().optional()
			}),
			annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: false, openWorldHint: true }
		},
		async ({ source, scene, build: wanted, id, type, title, toons, dry_run }): Promise<Result> => {
			try {
				if (!/^https?:\/\//i.test(source)) return fail('source must be http(s)');
				const base = source.replace(/\/+$/, '');
				// the content base's latest may be another publisher's (OSCT's content build): an editor's version is named
				const latest = wanted ? { build: wanted } : ((await (await download(`${base}/latest.json?t=${Date.now()}`)).json()) as Record<string, unknown>);
				const build = String(latest.build ?? '');
				if (!/^[0-9a-f]{6,64}$/.test(build)) return fail(`${base}/latest.json names no build – is this the content base of a published experience?`);
				const versionUrl = `${base}/${build}`;
				const raw = (await (await download(`${versionUrl}/game.config.json`)).json()) as Record<string, unknown>;
				const plan = planImport(raw, versionUrl, scene);
				const folder = `xr/${id}/${build}`;
				const outline = {
					build,
					folder,
					scenes: (plan.configuration.scenes as { id?: string }[]).map((s) => s.id),
					files: plan.files.length,
					plugins: plan.plugins.map((p) => `${p.id} (plugin API ${p.api})`),
					skipped_plugins: plan.skipped
				};
				if (dry_run) return text({ dry_run: true, ...outline, file_paths: plan.files.map((f) => f.path) });

				const plugins: XrPlugin[] = [];
				for (const plugin of plan.plugins) {
					const code = await (await download(plugin.from)).text();
					if (new TextEncoder().encode(code).length > PLUGIN_BYTES) return fail(`the plugin ${plugin.id} is larger than ${PLUGIN_BYTES} bytes`);
					plugins.push({ id: plugin.id, api: plugin.api, code });
				}

				let uploaded = 0;
				let kept = 0;
				for (const file of plan.files) {
					const blob = new Blob([await (await download(file.from)).arrayBuffer()], { type: mimeOf(file.path) });
					try {
						await assets.upload(blob, `${folder}/${file.path}`, { tags: ['xr', id] });
						uploaded++;
					} catch (error) {
						if (!(error instanceof ApiError) || error.status !== 409) throw new Error(`${file.path}: ${apiMessage(error)}`);
						kept++;
					}
				}

				const version = (raw.version ?? {}) as Record<string, unknown>;
				const origin = (raw.experience ?? {}) as Record<string, unknown>;
				const doc: XrExperience = {
					id,
					type,
					title,
					toons,
					assets: folder,
					source: {
						osct: { experience: String(origin.id ?? latest.experience ?? ''), ...(scene ? { scene } : {}), build, schema: String(version.schema ?? ''), app: String(version.version ?? '') }
					},
					configuration: plan.configuration as XrGameConfiguration,
					plugins
				};
				const key = `experience:${id}`;
				const newest = await content.getDraft(key).then(
					(wrapped) => wrapped.version.id,
					(error: unknown) => {
						if (error instanceof ApiError && error.status === 404) return null;
						throw error;
					}
				);
				const note = `OSCT ${doc.source?.osct.experience}${scene ? ` / ${scene}` : ''}, version ${build}`;
				const saved = await content.putDraft(key, doc, newest, config.token.startsWith('sct_') ? note : `${note} (agent)`);
				return text({
					saved: !saved.unchanged,
					version: saved.version.id,
					status: saved.version.status,
					...outline,
					files: { used: plan.files.length, uploaded, kept },
					open: `${options.links.viewer}/xr/${id}`,
					next: 'a draft: the member publishes experience:' + id + ' (with the draft preview on, the reader shows the draft at the link)'
				});
			} catch (error) {
				return fail(apiMessage(error));
			}
		}
	);
}
