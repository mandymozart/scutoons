// Developer API (BACKLOG 10.61): `/dev/*`, role `developer` – the database migrations, planned (a dry run) and applied
// by the developer's agents over MCP (their normal AI connection). The deploy runs them itself (/deploy/migrate).
import type { ApiClient } from './client.ts';

export type MigrationRunStatus = 'planned' | 'applied' | 'failed' | 'stale';
/** data: dry-run in a rolled-back transaction · ddl: commits by itself, listed only · mixed / other: not dry-run */
export type MigrationKind = 'data' | 'ddl' | 'mixed' | 'other';

export interface MigrationFile {
	name: string;
	/** null = pending */
	applied_at: string | null;
	sha256: string;
}

export interface MigrationStatement {
	/** the statement, cut at 400 characters */
	sql: string;
	kind: 'data' | 'ddl' | 'other';
	/** the rows the dry run changed; null = not dry-run */
	rows: number | null;
	/** an ADD COLUMN of a column that exists */
	skipped: boolean;
	/** false: the rows it changes can't be backed up first (several tables, REPLACE, ON DUPLICATE KEY) */
	backup: boolean;
}

export interface MigrationFilePlan {
	name: string;
	sha256: string;
	kind: MigrationKind;
	dry_run: boolean;
	/** the dry run failed (may depend on a DDL file above it, which the dry run doesn't run) */
	error: string | null;
	statements: MigrationStatement[];
}

export interface MigrationRunSummary {
	id: string;
	status: MigrationRunStatus;
	files: string[];
	/** files whose dry run failed */
	errors: number;
	/** deploy | session | ai:<the AI connection's name> */
	created_via: string;
	created_at: string;
	applied_at: string | null;
	error: string | null;
}

export interface MigrationRun extends MigrationRunSummary {
	plan: MigrationFilePlan[];
	/** null = the deploy */
	created_by: string | null;
	applied_by: string | null;
	log: string | null;
	/** the backup of the changed rows, below the server's storage/ */
	backup: string | null;
}

export interface MigrationStatus {
	status: 'success';
	database: string;
	base_schema: boolean;
	pending: number;
	files: MigrationFile[];
	runs: MigrationRunSummary[];
}

const seg = encodeURIComponent;

export function createDevApi(api: ApiClient) {
	return {
		migrations: () => api.get<MigrationStatus>('/dev/migrations'),
		plan: () => api.post<{ status: 'success'; run: MigrationRun }>('/dev/migrations/plan', {}),
		run: (id: string) => api.get<{ status: 'success'; run: MigrationRun }>(`/dev/migrations/runs/${seg(id)}`),
		apply: (id: string) => api.post<{ status: 'success'; run: MigrationRun }>(`/dev/migrations/runs/${seg(id)}/apply`, {}),
		/** plan + apply in one go; run null = nothing pending */
		migrate: () => api.post<{ status: 'success'; run: MigrationRun | null }>('/dev/migrations/run', {})
	};
}

export type DevApi = ReturnType<typeof createDevApi>;
