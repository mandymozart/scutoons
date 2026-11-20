// The History in the MCP (#687, after OSCT's #312): `revert_change` and `revert_to`. Both only make a NEW DRAFT version
// (append-only; the versions in between stay) – the member checks it in the creator and publishes there. Every rule is
// deck-core's: what a step is, what a revert undoes, what stays (places changed again since) – the core's wasm, here
// through `core-history.ts`; nothing is decided in this file.
import type { McpServer } from '@modelcontextprotocol/server';
import * as z from 'zod';
import { createApiClient, createContentApi, createHistory } from '@scutoons/api';
import type { HistoryView } from '@scutoons/content';
import type { Config } from './config.ts';
import { apiMessage } from './server.ts';
import { creatorHref, type AppLinks } from './services.ts';

type Result = { content: { type: 'text'; text: string }[]; isError?: boolean };
const text = (value: unknown): Result => ({ content: [{ type: 'text', text: typeof value === 'string' ? value : JSON.stringify(value, null, 1) }] });
const fail = (message: string): Result => ({ ...text(message), isError: true });

export function registerHistoryTools(server: McpServer, config: Config, options: { links: AppLinks; core: () => Promise<HistoryView>; fetch?: typeof fetch }): void {
	const content = createContentApi(createApiClient({ baseUrl: config.api, getToken: () => config.token, fetch: options.fetch }));
	const history = createHistory(content, options.core);
	const write = { readOnlyHint: false, destructiveHint: false, idempotentHint: false, openWorldHint: false };
	const guard =
		<A>(fn: (args: A) => Promise<Result>) =>
		async (args: A): Promise<Result> => {
			try {
				return await fn(args);
			} catch (error) {
				return fail(apiMessage(error));
			}
		};
	const newest = async (key: string, base: number | undefined) => base ?? (await content.getDraft(key)).version.id;

	server.registerTool(
		'revert_change',
		{
			description:
				'Takes back ONE earlier change of a stored document, keeping everything saved after it: `version` = any version of that change (list_versions; a burst of saves by one person under 10 seconds apart is one change – the core decides). The result is a NEW draft version, never a publish; places changed again since stay as they are and are named in `kept`. `base` = the version the revert applies to (default: the newest draft; a newer save in between → 409). The member checks the draft in the creator (`open`) and publishes there. A change can be reverted twice (the revert is itself a change).',
			inputSchema: z.object({
				key: z.string().describe('stored key (`chapter:catpope:2`, `events` …)'),
				version: z.number().int().describe('a version of the change to take back'),
				base: z.number().int().optional().describe('the version to apply it on (default: the newest draft)')
			}),
			annotations: write
		},
		guard(async ({ key, version, base }) => {
			const onto = await newest(key, base);
			const { steps } = await history.load(key);
			const step = steps.find((s) => version >= s.first && version <= s.last);
			if (!step) return fail(`version ${version} is not in the history of ${key} (list_versions)`);
			if (step.before === null) return fail(`version ${version} is the first one stored for ${key} – there is nothing before it to go back to (revert_to another version)`);
			const plan = await history.plan(key, step.before, step.last, onto);
			if (!plan.applied) return text({ reverted: false, reason: 'nothing to revert: the places this change touched were changed again since (or it is already undone)', kept: plan.kept, base: onto });
			// the API signs the draft "revert #<n>" (and "(AI: <connection>)" for a personal token)
			const id = await history.apply(key, step.last, plan);
			return text({ reverted: true, change: { first: step.first, last: step.last, versions: step.count, author: step.author }, places: plan.applied, kept: plan.kept, version: id, status: 'draft', open: creatorHref(options.links, key) });
		})
	);

	server.registerTool(
		'revert_to',
		{
			description:
				'Goes back to an earlier version of a stored document: its content becomes a NEW draft version (never a publish; the saves after it stay in the history, restorable). Unlike revert_change this undoes everything saved since – `later` says how many saves and `others` which other people made them. `base` = the version it replaces (default: the newest draft; a newer save in between → 409). The member checks the draft in the creator (`open`) and publishes there.',
			inputSchema: z.object({
				key: z.string(),
				version: z.number().int().describe('the version to go back to (list_versions)'),
				base: z.number().int().optional().describe('the version it replaces (default: the newest draft)')
			}),
			annotations: write
		},
		guard(async ({ key, version, base }) => {
			const onto = await newest(key, base);
			const { versions } = await history.load(key);
			if (!versions.some((v) => v.id === version)) return fail(`version ${version} is not in the history of ${key} (list_versions)`);
			// `me` is unknown to the MCP: `others` names everyone who saved after the point, this connection included
			const words = await history.backToWords(versions, version, '');
			const id = await history.backTo(key, version, onto);
			return text({ restored: version, later: words.later, others: words.others, version: id, status: 'draft', open: creatorHref(options.links, key) });
		})
	);
}
