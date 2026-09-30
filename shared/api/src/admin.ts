// Admin API (agents/ITEMS.md → "Admin API"): `/admin/*`, session + role `admin`. Framework-agnostic like the rest
// of the package; the app keeps list state in its URLs and passes the query through.
import type { ApiClient } from './client.ts';

/** URL-driven list query: `page` from 1, `page_size` (server clamps 1…500), `sort` = field | -field, `q`, filters */
export type ListParams = Record<string, string | number | boolean | null | undefined>;

export interface Pagination {
	total: number;
	page: number;
	pageSize: number;
}

export interface ListResult<T> {
	status: 'success';
	items: T[];
	pagination: Pagination;
}

export type EventName = 'scan' | 'access' | 'claim' | 'claim_refused' | 'login' | 'signup' | 'revoke' | 'unrevoke';

export const EVENT_NAMES: EventName[] = ['scan', 'access', 'claim', 'claim_refused', 'login', 'signup', 'revoke', 'unrevoke'];

export type EventCounts = Record<EventName, number>;

export interface DashboardEdition extends EventCounts {
	id: string | null;
	name: string | null;
	toon: string | null;
	edition_count: number | null;
	codes: number;
	claimed: number;
	revoked: number;
	signed: number;
}

export interface Dashboard {
	status: 'success';
	days: number;
	from: string;
	to: string;
	edition_id: string | null;
	totals: {
		codes: number;
		codes_claimed: number;
		codes_revoked: number;
		codes_signed: number;
		users: number;
		users_opted_in: number;
		users_new: number;
	};
	period: EventCounts & { unique_devices: number };
	editions: DashboardEdition[];
	daily: (EventCounts & { day: string })[];
	daily_by_edition: (EventCounts & { day: string; edition_id: string | null })[];
}

/** Times of codes are the qr table's server-local `YYYY-MM-DD HH:MM:SS` */
export interface AdminCodeRow {
	code: string;
	type: string;
	edition_id: string | null;
	edition_number: number | null;
	created_at: string | null;
	claimed: boolean;
	claimed_at: string | null;
	expires_at: string | null;
	revoked_at: string | null;
	revoked_reason: string | null;
	scans: number;
	accesses: number;
	devices: number;
	logins: number;
	refused: number;
	first_seen: string | null;
	last_seen: string | null;
	signed: boolean;
}

export interface AdminCodeDetail {
	status: 'success';
	code: {
		code: string;
		type: string;
		edition_id: string | null;
		edition_number: number | null;
		redirect_to: string;
		created_at: string | null;
		claimed_at: string | null;
		expires_at: string | null;
		revoked_at: string | null;
		revoked_reason: string | null;
		claimed: boolean;
	};
	items: { item_id: string; variant: string | null; qty: number }[];
	options: { item_id: string; variants: string[] | null }[];
	edition: { id: string; name: string; number: number | null; count: number | null } | null;
	stats: {
		scans: number;
		accesses: number;
		devices: number;
		logins: number;
		refused: number;
		first_seen: string | null;
		last_seen: string | null;
	};
	signed: boolean;
	expired: boolean;
	revoked: boolean;
}

export interface CodeEvent {
	event: EventName;
	created_at: string;
	device: string | null;
	account: boolean;
	toon: string | null;
}

/** The client-side ML-DSA signature stored with a code (made by the admin app / legacy generate) */
export interface CodeSignature {
	signature: string;
	publicKey: string;
	algorithm: string;
	signedData: string;
	timestamp: string;
}

export interface CodeSignatureResult {
	status: 'success';
	signature: CodeSignature;
	fingerprint: string;
	/** `signedData` parsed: code, type, url, redirect_to, dates and the certificate template fields */
	qr_data: Record<string, any> | null;
	redirect_to: string;
}

export interface CreateCodeInput {
	redirect_to: string;
	type: string;
	/** omitted → the server makes a fresh nanoid(8) */
	code?: string;
	email?: string;
	expires_at?: string | null;
	edition_id?: string | null;
}

export interface CreatedCode {
	code: string;
	redirect_to: string;
	type: string;
	email: string;
	expires_at: string | null;
	edition_id: string | null;
}

export type BatchStep = 'db' | 'svg' | 'pdf' | 'sig';

export interface BatchManifestCode {
	n: number;
	code: string;
	status: 'pending' | 'running' | 'done' | 'error';
	completedSteps: BatchStep[];
}

/** `BATCH-<UTC timestamp>-<edition id>.json` (same format as the legacy generate/ batches) */
export interface BatchManifest {
	filename: string;
	edition: string;
	editionId: string;
	productname: string;
	edition_number: number;
	edition_count: number;
	excluded_number: number;
	creators: string;
	redirectUrl: string;
	total: number;
	created_at: string;
	codes: BatchManifestCode[];
	[key: string]: unknown;
}

export interface BatchRow {
	filename: string;
	edition: string;
	editionId: string;
	total: number;
	done: number;
	errors: number;
	created_at: string;
	complete: boolean;
}

export type FileType = 'svgs' | 'certificates' | 'signatures' | 'batch' | 'zips';

export interface AdminUserRow {
	id: string;
	email: string;
	created_at: string;
	roles: string[];
	signup_code: string | null;
	marketing_opt_in: boolean;
	marketing_opt_in_at: string | null;
	codes: number;
}

export interface AdminUserDetail extends Omit<AdminUserRow, 'codes'> {
	locale: string | null;
	codes: {
		code: string;
		type: string;
		edition_id: string | null;
		edition_number: number | null;
		claimed_at: string | null;
		revoked_at: string | null;
	}[];
	inventory_entries: number;
	sessions: number;
	last_seen_at: string | null;
	/** avatar-body unlocks: the defaults (source `default`) + granted ones */
	characters?: AdminCharacterUnlock[];
}

export interface AdminCharacterUnlock {
	character: string;
	looks: string[];
	source: 'default' | 'code' | 'item' | 'story' | 'admin';
	ref: string | null;
	unlocked_at: string | null;
}

export interface RoleInfo {
	id: string;
	name: string;
	description: string;
}

export interface AdminEventRow {
	id: number;
	code: string;
	event: EventName;
	created_at: string;
	device: string | null;
	account: boolean;
	toon: string | null;
	edition_id: string | null;
}

export interface AdminCatalog {
	status: 'success';
	/** = GET /items/ */
	catalog: { version?: number; slots?: string[]; items: Record<string, any>[]; recipes?: Record<string, any>[] } & Record<string, any>;
	editions: Record<string, any>[];
	toons: { id: string; name: string; redirectUrl: string }[];
	/** the server-only permissions map, read-only */
	permissions: Record<string, any>;
}

export interface AdminKeys {
	status: 'success';
	algorithm: string;
	publicKey: string;
	privateKey: string;
	generated: string | null;
}

/** One stored file of the content store (every toon), see ITEMS.md "Admin API" → Content */
/** One stored entity (entity key, agents/DATA.md) of the content store */
export interface AdminContentEntity {
	key: string;
	kind: string;
	/** null = platform-wide (characters, people, events …) */
	toon: string | null;
	/** null = language-neutral */
	locale: string | null;
	latest: number;
	latest_status: 'draft' | 'published' | string;
	latest_at: string;
	published: number | null;
	published_at: string | null;
	versions: number;
}

export interface AdminContentIndex {
	status: 'success';
	toons: { id: string; name: string }[];
	entities: AdminContentEntity[];
}

/** A runtime setting (table `settings`), see ITEMS.md "Admin API" → Settings. 0 = no limit for the limits. */
export interface AdminSetting {
	key: 'scan_limit_per_hour' | 'lookup_limit_per_hour' | (string & {});
	value: number;
	/** the config value used when nothing is stored */
	default: number;
	min: number;
	max: number;
	/** a value is stored (else the default applies) */
	stored: boolean;
	updated_at: string | null;
	/** `admin:<email>` of the last change */
	updated_by: string | null;
}

export interface AdminSettings {
	status: 'success';
	items: AdminSetting[];
	/** PUT only: the keys that changed */
	changed?: string[];
}

/** `?a=1&b=x` from params (empty / null / undefined dropped) */
export function toQuery(params: ListParams = {}): string {
	const search = new URLSearchParams();
	for (const [key, value] of Object.entries(params)) {
		if (value === undefined || value === null || value === '') continue;
		search.set(key, String(value));
	}
	const text = search.toString();
	return text ? `?${text}` : '';
}

const seg = encodeURIComponent;

export type AdminApi = ReturnType<typeof createAdminApi>;

export function createAdminApi(api: ApiClient) {
	return {
		dashboard: (params: { days?: number; edition_id?: string | null } = {}) =>
			api.get<Dashboard>(`/admin/dashboard${toQuery(params)}`),

		listCodes: (params: ListParams = {}) => api.get<ListResult<AdminCodeRow>>(`/admin/codes${toQuery(params)}`),
		/** CSV of the filtered list (no paging) */
		codesCsv: (params: ListParams = {}) =>
			api.fetchRaw(`/admin/codes${toQuery({ ...params, page: undefined, page_size: undefined, format: 'csv' })}`),
		createCode: (input: CreateCodeInput) => api.post<{ status: string; code: CreatedCode }>('/admin/codes', input),
		getCode: (code: string) => api.get<AdminCodeDetail>(`/admin/codes/${seg(code)}`),
		codeEvents: (code: string, limit = 1000) =>
			api.get<{ status: string; items: CodeEvent[] }>(`/admin/codes/${seg(code)}/events?limit=${limit}`),
		revokeCode: (code: string, reason = '') =>
			api.post<{ status: string; code: string; revoked_at: string | null; revoked_reason: string | null }>(
				`/admin/codes/${seg(code)}/revoke`,
				{ reason }
			),
		unrevokeCode: (code: string) =>
			api.post<{ status: string; code: string; revoked_at: null; revoked_reason: null }>(
				`/admin/codes/${seg(code)}/unrevoke`
			),
		getSignature: (code: string) => api.get<CodeSignatureResult>(`/admin/codes/${seg(code)}/signature`),
		saveSignature: (code: string, signature: CodeSignature) =>
			api.put<{ status: string; code: string }>(`/admin/codes/${seg(code)}/signature`, { signature }),

		listBatches: (params: ListParams = {}) => api.get<ListResult<BatchRow>>(`/admin/batches${toQuery(params)}`),
		/** Reserves unique codes in a new manifest (not inserted). 409 batch_complete → `force: true` */
		createBatch: (input: { edition_id: string; count?: number; start_number?: number; force?: boolean }) =>
			api.post<{ status: string; batch: BatchManifest }>('/admin/batches', input),
		getBatch: (filename: string) => api.get<{ status: string; batch: BatchManifest }>(`/admin/batches/${seg(filename)}`),
		saveBatch: (manifest: BatchManifest) =>
			api.put<{ status: string; batch: BatchManifest }>(`/admin/batches/${seg(manifest.filename)}`, { manifest }),
		batchZip: (filename: string) => api.fetchRaw(`/admin/batches/${seg(filename)}/zip`),
		/** `doc`: SVG markup, a PDF data URI or JSON text */
		putFile: (type: FileType, filename: string, doc: string) =>
			api.put<{ status: string; type: FileType; filename: string }>(`/admin/files/${type}/${seg(filename)}`, { doc }),
		getFile: (type: FileType, filename: string) => api.fetchRaw(`/admin/files/${type}/${seg(filename)}`),

		listUsers: (params: ListParams = {}) => api.get<ListResult<AdminUserRow>>(`/admin/users${toQuery(params)}`),
		getUser: (id: string) => api.get<{ status: string; user: AdminUserDetail }>(`/admin/users/${seg(id)}`),
		grantRole: (id: string, role: string) =>
			api.put<{ status: string; changed: boolean; user: AdminUserDetail }>(`/admin/users/${seg(id)}/roles/${seg(role)}`),
		revokeRole: (id: string, role: string) =>
			api.delete<{ status: string; changed: boolean; user: AdminUserDetail }>(
				`/admin/users/${seg(id)}/roles/${seg(role)}`
			),
		roles: () => api.get<{ status: string; items: RoleInfo[] }>('/admin/roles'),
		/** unlock an avatar-body character (+ further A-pose looks) for an account */
		unlockCharacter: (id: string, character: string, body: { looks?: string[]; source?: AdminCharacterUnlock['source']; ref?: string } = {}) =>
			api.put<{ status: string; changed: boolean; characters: AdminCharacterUnlock[] }>(`/admin/users/${seg(id)}/characters/${seg(character)}`, body),
		/** revoke a character unlock (with its looks), or one look */
		lockCharacter: (id: string, character: string, look?: string) =>
			api.delete<{ status: string; changed: boolean; characters: AdminCharacterUnlock[] }>(
				`/admin/users/${seg(id)}/characters/${seg(character)}${look ? `/looks/${seg(look)}` : ''}`
			),

		listEvents: (params: ListParams = {}) => api.get<ListResult<AdminEventRow>>(`/admin/events${toQuery(params)}`),
		catalog: () => api.get<AdminCatalog>('/admin/catalog'),
		keys: () => api.get<AdminKeys>('/admin/keys'),
		content: (params: { toon?: string } = {}) => api.get<AdminContentIndex>(`/admin/content${toQuery(params)}`),

		settings: () => api.get<AdminSettings>('/admin/settings'),
		/** key → whole number (min…max), or null = back to the default */
		saveSettings: (values: Record<string, number | null>) => api.put<AdminSettings>('/admin/settings', values)
	};
}
