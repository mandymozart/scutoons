// The database migrations (BACKLOG 10.61) for a developer's agents: listed for the role developer (their normal AI
// connection). Only the files that came with the deploy run (server/migrations/mysql/) – never SQL of the agent's own.
// plan_migrations is a dry run (data statements rolled back, row counts); apply_migrations runs exactly that plan
// (the files unchanged since, within a day), backs up the changed rows and empties the library cache. The deploy runs
// pending migrations by itself, so these are for checking and for what a deploy didn't run.
import type { McpServer } from '@modelcontextprotocol/server';
import * as z from 'zod';
import { createApiClient, createDevApi, type MigrationRun } from '@scutoons/api';
import type { Config } from './config.ts';
import { apiMessage } from './server.ts';

type Result = { content: { type: 'text'; text: string }[]; isError?: boolean };
const text = (value: unknown): Result => ({ content: [{ type: 'text', text: typeof value === 'string' ? value : JSON.stringify(value, null, 1) }] });
const guard =
	<A>(fn: (args: A) => Promise<Result>) =>
	async (args: A): Promise<Result> => {
		try {
			return await fn(args);
		} catch (error) {
			return { ...text(apiMessage(error)), isError: true };
		}
	};

/** a plan as the agent needs it: per file its kind, the dry run's rows or error, the statements */
function planView(run: MigrationRun) {
	return run.plan.map((f) => ({
		file: f.name,
		kind: f.kind,
		dry_run: f.dry_run ? (f.error ? `FAILED: ${f.error}` : 'ok') : 'not run (schema changes commit by themselves)',
		statements: f.statements.map((s) => ({
			sql: s.sql,
			kind: s.kind,
			...(s.rows !== null ? { rows: s.rows } : {}),
			...(s.skipped ? { skipped: 'the column exists' } : {}),
			...(s.kind === 'data' && !s.backup ? { backup: 'none (several tables / REPLACE / ON DUPLICATE KEY)' } : {})
		}))
	}));
}

export function registerDevTools(server: McpServer, config: Config): void {
	const dev = createDevApi(createApiClient({ baseUrl: config.api, getToken: () => config.token }));

	server.registerTool(
		'migration_status',
		{
			description:
				'The database migrations (server/migrations/mysql/): which database, the pending files (not in schema_migrations yet) and the latest runs (planned / applied / failed / stale, with their log). Read-only.',
			inputSchema: z.object({ all: z.boolean().optional().describe('true: also every applied file with its date') }),
			annotations: { readOnlyHint: true, openWorldHint: false }
		},
		guard(async ({ all }: { all?: boolean }) => {
			const s = await dev.migrations();
			return text({
				database: s.database,
				pending: s.files.filter((f) => f.applied_at === null).map((f) => f.name),
				...(all ? { applied: s.files.filter((f) => f.applied_at !== null).map((f) => `${f.name} (${f.applied_at})`) } : { applied: s.files.length - s.pending }),
				runs: s.runs
			});
		})
	);

	server.registerTool(
		'plan_migrations',
		{
			description:
				'Plans the pending migrations: a dry run on the database – each file of data statements runs in a transaction that is rolled back, so you see how many rows every statement would change; files with schema changes (DDL) are only listed. Nothing changes. Returns the plan with its id for apply_migrations; a new plan makes older open ones stale.',
			inputSchema: z.object({}),
			annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: false, openWorldHint: false }
		},
		guard(async () => {
			const { run } = await dev.plan();
			if (!run.plan.length) return text({ id: run.id, plan: [], note: 'Nothing is pending.' });
			return text({ id: run.id, plan: planView(run), next: 'Check the row counts, then apply_migrations with this id.' });
		})
	);

	server.registerTool(
		'apply_migrations',
		{
			description:
				'Applies a plan from plan_migrations to the database: only while its files are unchanged and pending (else plan again), within a day. A data-only file runs all or nothing; a file with schema changes statement by statement. The rows each UPDATE / DELETE changes are backed up first (server storage/migration-backups); the library cache is emptied afterwards. Returns the status (applied / failed) and the log.',
			inputSchema: z.object({ id: z.string().uuid().describe('the plan id from plan_migrations') }),
			annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: false }
		},
		guard(async ({ id }: { id: string }) => {
			const { run } = await dev.apply(id);
			return text({ status: run.status, files: run.files, error: run.error, log: run.log, backup: run.backup });
		})
	);
}
