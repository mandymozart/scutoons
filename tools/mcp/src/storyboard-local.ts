// `generate_local` (#127): a still for one shot from the member's own ComfyUI (comfy-local.ts) – the shot's prompt
// (content's lumaPrompt, or the member's), image 1 = the shot's render (found on this machine or given), up to two
// guides (the figures' character images) – saved into SCUTOONS_WORKSPACE/<toon>/<chapter>/<shot>--g<n>.png and recorded
// (generator comfy-local, the machine, the file, its sha256). Free: no budget, no server job. A slow GPU (an 8 GB card
// takes ~10 min a shot) outlasts one tool call: the call waits up to `wait` seconds, else answers `running` with the
// prompt id, and the same tool with `prompt_id` collects it later.
import { createHash } from 'node:crypto';
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { basename, resolve } from 'node:path';
import { hostname } from 'node:os';
import type { McpServer } from '@modelcontextprotocol/server';
import * as z from 'zod';
import { createApiClient, createContentApi } from '@scutoons/api';
import { lumaPrompt, nextName, storyboardKey, type ChapterStoryboard } from '@scutoons/content/storyboard';
import {
	comfyFetchImage,
	comfyFree,
	comfyGraph,
	comfyQueue,
	comfyReachable,
	comfySettings,
	comfyUpload,
	comfyWait,
	notReachable,
	pickInputs,
	recordParams,
	type LocalInput
} from './comfy-local.ts';
import type { Config } from './config.ts';
import { apiMessage } from './server.ts';
import type { AppLinks } from './services.ts';
import { compactGeneration, locateShot, machineLabel, recordBody, resultDir, shotIds, storyboardHref, type Generation } from './storyboard-model.ts';
import { shotReferences, studioFolders } from './storyboard-refs.ts';

type Result = { content: { type: 'text'; text: string }[]; isError?: boolean };
const text = (value: unknown): Result => ({ content: [{ type: 'text', text: typeof value === 'string' ? value : JSON.stringify(value) }] });
const fail = (message: string): Result => ({ ...text(message), isError: true });

/** what a run needs to be recorded when it is collected later (this process only – after a restart: a shorter record) */
interface Pending {
	prompt: string;
	inputs: LocalInput[];
	seed: number;
	params: Record<string, unknown>;
	t0: number;
}
const pending = new Map<string, Pending>();

export function registerLocalGenerationTools(server: McpServer, config: Config, options: { links: AppLinks }): void {
	const client = createApiClient({ baseUrl: config.api, getToken: () => config.token });
	const content = createContentApi(client);
	const open = (toon: string, chapter: number) => storyboardHref(options.links.creator, toon, chapter);

	server.registerTool(
		'generate_local',
		{
			description:
				'A still for one shot from the member\'s OWN ComfyUI on this computer (COMFY_URL, default http://127.0.0.1:8188; Qwen-Image-Edit 2509 + Lightning). Free – no budget, nothing sent to SCUTOONS but the record. Image 1 = the picture edited: `still` (a local PNG – e.g. the 3D view exported from the storyboard page), else the shot\'s render found on this machine; `guides` = up to two more images (default: the figures\' character images found here). Prompt: the shot\'s (storyboard_prompt) unless given. The result goes into SCUTOONS_WORKSPACE/<toon>/<chapter>/<shot>--g<n>.png and is recorded (generator comfy-local, this machine, the file, its sha256). A slow GPU: the call waits up to `wait` s (default 50), else answers `running` with a prompt_id – call it again with that prompt_id (same toon / chapter / shot) to collect.',
			inputSchema: z.object({
				toon: z.string().regex(/^[a-z0-9-]{1,64}$/),
				chapter: z.number().int().min(1),
				shot: z.string().regex(/^[\w.-]{1,64}$/),
				still: z.string().optional().describe('local image file = image 1'),
				guides: z.array(z.string()).max(2).optional().describe('local image files = images 2–3'),
				prompt: z.string().max(20000).optional(),
				seed: z.number().int().min(0).optional(),
				steps: z.number().int().min(1).max(100).optional().describe('default 4 (Lightning); 40 with COMFY_LORA=none on a big GPU'),
				cfg: z.number().min(0).max(20).optional(),
				wait: z.number().int().min(0).max(280).optional().describe('seconds to wait for the image in this call (default 50)'),
				prompt_id: z.string().max(64).optional().describe('collect a run started earlier'),
				note: z.string().max(1000).optional()
			}),
			annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: false, openWorldHint: false }
		},
		async (a) => {
			try {
				const workspace = process.env.SCUTOONS_WORKSPACE;
				if (!workspace) return fail('Set the workspace folder first (the extension\'s setting "Workspace folder", or SCUTOONS_WORKSPACE) – results are saved there.');
				const m = comfySettings(process.env, { steps: a.steps, cfg: a.cfg });
				const down = await comfyReachable(m.url);
				if (down) return fail(notReachable(m.url, down));
				const host = new URL(m.url).host;

				let promptId = a.prompt_id;
				let run = promptId ? pending.get(promptId) : undefined;
				if (!promptId) {
					const doc = (await content.getDraft<ChapterStoryboard>(storyboardKey(a.toon, a.chapter))).content;
					const found = locateShot(doc, a.shot);
					if (!found) return fail(`no shot ${a.shot} in ${storyboardKey(a.toon, a.chapter)} (shots: ${shotIds(doc).join(', ')})`);
					const refs = shotReferences(studioFolders(), a.toon, a.chapter, found.scene, found.shot);
					const picked = pickInputs({ still: a.still, guides: a.guides }, refs);
					if ('error' in picked) return fail(picked.error);
					const prompt = a.prompt ?? lumaPrompt(found.scene, found.shot, { references: { structure: picked.inputs[0].file, characters: picked.inputs.slice(1).map((i) => i.file), style: [] } });
					const seed = a.seed ?? Math.floor(Math.random() * 2 ** 31);
					const names: string[] = [];
					for (const input of picked.inputs) names.push(await comfyUpload(m.url, `scutoons-${a.shot}-${basename(input.file)}`, await readFile(input.file)));
					promptId = await comfyQueue(m.url, comfyGraph(prompt, names, seed, m));
					run = { prompt, inputs: picked.inputs, seed, params: recordParams(m, host), t0: Date.now() };
					pending.set(promptId, run);
				}

				const done = await comfyWait(m.url, promptId, (a.wait ?? 50) * 1000);
				if (!done) return text({ running: promptId, next: `generate_local { toon: "${a.toon}", chapter: ${a.chapter}, shot: "${a.shot}", prompt_id: "${promptId}" } in a minute or two (a small GPU takes ~10 min a shot)` });
				if ('error' in done) {
					pending.delete(promptId);
					return fail(`ComfyUI failed: ${done.error}`);
				}
				const bytes = await comfyFetchImage(m.url, done.image);
				await comfyFree(m.url);
				pending.delete(promptId);

				const dir = resolve(...resultDir(workspace, a.toon, a.chapter));
				await mkdir(dir, { recursive: true });
				const listed = await client.get<{ items: Generation[] }>(`/storyboard/generations?${new URLSearchParams({ toon: a.toon, chapter: String(a.chapter), shot: a.shot })}`);
				const taken = [...(await readdir(dir)), ...listed.items.map((g) => String(g.file ?? ''))];
				const ext = /\.jpe?g$/i.test(done.image.filename) ? 'jpg' : /\.webp$/i.test(done.image.filename) ? 'webp' : 'png';
				const file = nextName(a.shot, taken, ext).file!;
				const path = resolve(dir, file);
				await writeFile(path, bytes, { flag: 'wx' });
				const hash = createHash('sha256').update(bytes).digest('hex');
				const { item } = await client.post<{ item: Generation }>(
					'/storyboard/generations',
					recordBody({
						toon: a.toon,
						chapter: a.chapter,
						shot: a.shot,
						file,
						file_hash: hash,
						machine: machineLabel(process.env, hostname()),
						generator: 'comfy-local',
						kind: 'image',
						model: 'qwen-image-edit',
						mode: 'comfy',
						status: 'succeeded',
						prompt: run?.prompt,
						seed: run?.seed,
						inputs: run?.inputs.map((i) => ({ role: i.role, name: basename(i.file) })),
						params: run?.params ?? recordParams(m, host),
						ms: run ? Date.now() - run.t0 : undefined,
						cost: 0,
						note: a.note
					})
				);
				return text({ saved: path, recorded: item.id, item: compactGeneration(item), open: open(a.toon, a.chapter) });
			} catch (error) {
				return fail(apiMessage(error));
			}
		}
	);
}
