// Admin API (agents/ITEMS.md → "Admin API"): `/admin/*`, session + role `admin`. Framework-agnostic like the rest
// of the package; the app keeps list state in its URLs and passes the query through.
import type { ApiClient } from './client.ts';
import type { FeaturedProduct, ItemRespawn } from '@scutoons/content';

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

/** The editions on the landing page (admin page "Startseite", `GET/PUT /admin/featured`): `[{ product, variant? }]` in order */
/** When picked-up placed items reappear, per item (`GET /admin/catalog/respawn`, agents/ITEMS.md "Placed items" → "Respawn") */
export interface AdminRespawn {
	status: 'success';
	items: {
		id: string;
		/** the rule of the catalog seed (server config/items.json; null = never) */
		file: ItemRespawn | null;
		/** the admin's override (null = never; only meaningful with `overridden`) */
		override: ItemRespawn | null;
		overridden: boolean;
		/** what applies (null = never reappears) */
		effective: ItemRespawn | null;
	}[];
	updated_at: string | null;
	updated_by: string | null;
	/** PUT only */
	changed?: boolean;
}

export interface AdminFeatured {
	status: 'success';
	/** the effective list (the stored one, else the default) */
	items: FeaturedProduct[];
	/** server config/featured-products.json, used while nothing is stored */
	default: FeaturedProduct[];
	stored: boolean;
	updated_at: string | null;
	/** `admin:<email>` of the last change */
	updated_by: string | null;
	/** the most items the list may have */
	max: number;
	/** PUT only */
	changed?: boolean;
}

// ── Ownership claims + transfers (agents/ITEMS.md "Ownership, claims, gifting") ──────────────────────────

/** An account as the ownership views show it (admin only); `email` null = the account was deleted */
export interface AdminAccountRef {
	id: string;
	email: string | null;
}

export type ClaimStatus = 'open' | 'transferred' | 'rejected' | 'withdrawn';

/** `GET /admin/claims` row */
export interface AdminClaimRow {
	id: string;
	code: string;
	status: ClaimStatus;
	claimant: AdminAccountRef;
	/** the owner when the claim was filed (null = registered by the old email-only form) */
	owner: AdminAccountRef | null;
	/** the code is in another account now than when the claim was filed */
	owner_changed: boolean;
	edition_id: string | null;
	edition_number: number | null;
	files: number;
	/** the first 140 characters */
	statement: string;
	created_at: string;
	decided_at: string | null;
}

export interface AdminClaimFile {
	id: string;
	name: string;
	type: string;
	size: number;
	created_at: string;
}

export interface AdminClaim {
	id: string;
	code: string;
	status: ClaimStatus;
	statement: string;
	claimant: AdminAccountRef | null;
	owner: AdminAccountRef | null;
	/** who decided */
	admin: AdminAccountRef | null;
	admin_note: string | null;
	files: AdminClaimFile[];
	created_at: string;
	decided_at: string | null;
}

/** One ownership change of a code (`item_transfers`): a claim decided, a gift, an admin transfer */
export interface OwnershipTransfer {
	id: number;
	kind: 'claim' | 'gift' | 'admin';
	code: string | null;
	item_id: string | null;
	variant: string | null;
	qty: number;
	from: AdminAccountRef | null;
	to: AdminAccountRef | null;
	/** the admin (claim, admin) or the giver (gift) */
	by: AdminAccountRef | null;
	claim_id: string | null;
	note: string | null;
	created_at: string;
}

/** `GET /admin/claims/{id}` */
export interface AdminClaimDetail {
	status: 'success';
	claim: AdminClaim;
	/** the code detail (as `GET /admin/codes/{code}`), null when the code is gone */
	code: Omit<AdminCodeDetail, 'status'> | null;
	current_owner: AdminAccountRef | null;
	/** registered by the old email-only form (no account) */
	legacy_registration: boolean;
	/** the server-side certificate check (`GET /api/certificates/{code}` without `status`), or `{ valid: false, reason, error }` */
	certificate: { valid: boolean; reason: string | null; registered?: boolean | null; revoked?: boolean | null; certificate?: Record<string, any> | null; error?: string };
	history: OwnershipTransfer[];
	other_claims: AdminClaim[];
}

/** One registered email template (server `Domain\Mail\MailTemplates`), `GET /admin/emails/templates` */
export interface AdminEmailTemplate {
	id: 'login_code' | 'signup_without_code' | (string & {});
	/** member = to a reader / account; admin = to the admin recipients */
	audience: 'member' | 'admin' | (string & {});
	description: string;
	/** variable name → type (string, url, int, bool, ?string = optional) */
	vars: Record<string, string>;
	/** the values the preview uses */
	sample: Record<string, unknown>;
}

export interface AdminEmailTemplates {
	status: 'success';
	locales: string[];
	/** MAIL_TRANSPORT of the API: smtp | log | mail */
	transport: string;
	items: AdminEmailTemplate[];
}

/** A template rendered with its sample vars, `GET /admin/emails/templates/{id}/preview?locale=` */
export interface AdminEmailPreview {
	status: 'success';
	id: string;
	locale: string;
	subject: string;
	text: string;
	html: string;
}

/** One row of the sent-mail log (table `mail_log`), `GET /admin/emails/log` */
export interface AdminEmailLogRow {
	id: number;
	template: string;
	recipient: string;
	locale: string;
	status: 'sent' | 'failed';
	transport: string;
	error: string | null;
	created_at: string;
}

/** The admin notification addresses (setting `mail_admin_recipients`), `GET/PUT /admin/emails/recipients` */
export interface AdminEmailRecipients {
	status: 'success';
	/** the effective list (stored, else the default) */
	items: string[];
	/** server MAIL_ADMIN_TO */
	default: string[];
	stored: boolean;
	updated_at: string | null;
	/** `admin:<email>` of the last change */
	updated_by: string | null;
	max: number;
	/** PUT only */
	changed?: boolean;
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

/** The catalog kinds the admin edits (`/admin/catalog/<kind>`, server docs/API.md "Catalog", agents/CATALOG.md) */
export const CATALOG_KINDS = ['items', 'recipes', 'albums', 'products', 'editions', 'featured', 'permissions', 'code-types', 'toons', 'meta'] as const;
export type CatalogKind = (typeof CATALOG_KINDS)[number];

/** One stored catalog entry: its definition (`data`, the shape of the old config file's entry) and order */
export interface CatalogEntry<T = Record<string, any>> {
	id: string;
	position: number;
	data: T;
	updated_at: string;
	updated_by: string | null;
}

export interface CatalogHistoryEntry {
	id: number;
	kind: CatalogKind;
	/** '' = the whole kind (reorder / replace) */
	entry_id: string;
	action: 'create' | 'update' | 'delete' | 'reorder' | 'replace';
	before: unknown;
	after: unknown;
	position: number | null;
	undo_of: number | null;
	note: string | null;
	by: string | null;
	at: string;
}

/** A 422 `invalid_catalog` answer's problems (also on 409 `in_use`: `uses`) */
export interface CatalogProblem {
	path?: (string | number)[];
	message: string;
	[key: string]: unknown;
}

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
		saveSettings: (values: Record<string, number | null>) => api.put<AdminSettings>('/admin/settings', values),

		respawn: () => api.get<AdminRespawn>('/admin/catalog/respawn'),
		/** an item's rule (an update of the item, catalog_history), null = never reappears, `reset` = back to the seed's rule */
		saveRespawn: (item: string, change: { respawn: ItemRespawn | null } | { reset: true }) =>
			api.put<AdminRespawn>(`/admin/catalog/respawn/${seg(item)}`, change),

		// the catalog in the database (CUTOVER 10.29): list / get / put (create 201 or update) / delete / reorder per kind,
		// every write validated (422 invalid_catalog { problems }) and recorded in the history (undo)
		catalogList: <T = Record<string, any>>(kind: CatalogKind) =>
			api.get<{ status: 'success'; kind: CatalogKind; items: CatalogEntry<T>[]; fallback: boolean }>(`/admin/catalog/${seg(kind)}`),
		catalogGet: <T = Record<string, any>>(kind: CatalogKind, id: string) =>
			api.get<{ status: 'success'; entry: CatalogEntry<T> }>(`/admin/catalog/${seg(kind)}/${seg(id)}`),
		catalogPut: <T = Record<string, any>>(kind: CatalogKind, id: string, data: T, note?: string) =>
			api.put<{ status: 'success'; created: boolean; changed: boolean; entry: CatalogEntry<T>; history_id: number | null }>(
				`/admin/catalog/${seg(kind)}/${seg(id)}`,
				{ data, ...(note ? { note } : {}) }
			),
		catalogDelete: (kind: CatalogKind, id: string) =>
			api.delete<{ status: 'success'; deleted: boolean; history_id: number | null }>(`/admin/catalog/${seg(kind)}/${seg(id)}`),
		catalogReorder: (kind: CatalogKind, ids: string[]) =>
			api.post<{ status: 'success'; changed: boolean; ids: string[]; history_id: number | null }>(`/admin/catalog/${seg(kind)}/reorder`, { ids }),
		catalogHistory: (params: { kind?: CatalogKind; id?: string; limit?: number } = {}) =>
			api.get<{ status: 'success'; items: CatalogHistoryEntry[] }>(`/admin/catalog/history${toQuery(params)}`),
		catalogUndo: (historyId: number, force = false) =>
			api.post<{ status: 'success'; undone: boolean; history_id: number | null; entry?: CatalogEntry; ids?: string[] }>(
				`/admin/catalog/history/${historyId}/undo`,
				force ? { force: true } : {}
			),

		featured: () => api.get<AdminFeatured>('/admin/featured'),
		/** the new ordered list, or null = back to the default */
		saveFeatured: (items: FeaturedProduct[] | null) => api.put<AdminFeatured>('/admin/featured', { items }),

		emailTemplates: () => api.get<AdminEmailTemplates>('/admin/emails/templates'),
		/** a template rendered with its sample vars */
		emailPreview: (id: string, locale: string) =>
			api.get<AdminEmailPreview>(`/admin/emails/templates/${seg(id)}/preview${toQuery({ locale })}`),
		/** the sent-mail log: filters template, status (sent|failed), recipient, from, to; q = part of the address */
		listEmails: (params: ListParams = {}) => api.get<ListResult<AdminEmailLogRow>>(`/admin/emails/log${toQuery(params)}`),
		emailRecipients: () => api.get<AdminEmailRecipients>('/admin/emails/recipients'),
		/** the new list, or null = back to the default (MAIL_ADMIN_TO) */
		saveEmailRecipients: (items: string[] | null) => api.put<AdminEmailRecipients>('/admin/emails/recipients', { items }),

		// ownership claims + transfers (agents/ITEMS.md "Ownership, claims, gifting")
		/** filters status (open|transferred|rejected|withdrawn), code, user (claimant or owner id); q = code or claimant email; `open` = open claims in all */
		listClaims: (params: ListParams = {}) => api.get<ListResult<AdminClaimRow> & { open: number }>(`/admin/claims${toQuery(params)}`),
		getClaim: (id: string) => api.get<AdminClaimDetail>(`/admin/claims/${seg(id)}`),
		/** move the code to the claimant (note required); mails both owners */
		transferClaim: (id: string, note: string, choice: { item_id?: string; variant?: string } = {}) =>
			api.post<AdminClaimDetail>(`/admin/claims/${seg(id)}/transfer`, { note, ...choice }),
		/** keep it where it is (note required, stays internal); mails the claimant */
		rejectClaim: (id: string, note: string) => api.post<AdminClaimDetail>(`/admin/claims/${seg(id)}/reject`, { note }),
		/** an evidence file (image / PDF) – inline, admin only */
		claimFile: (id: string, fileId: string) => api.fetchRaw(`/admin/claims/${seg(id)}/files/${seg(fileId)}`),
		/** the ownership history of a code, newest first */
		codeHistory: (code: string) => api.get<{ status: string; items: OwnershipTransfer[] }>(`/admin/codes/${seg(code)}/history`),
		/** move a code to another account outside a claim: `to` = email or account id, note required */
		transferCode: (code: string, input: { to: string; note: string; item_id?: string; variant?: string }) =>
			api.post<{ status: string; code: string; from: AdminAccountRef | null; to: AdminAccountRef; entries: number; history: OwnershipTransfer[] }>(
				`/admin/codes/${seg(code)}/transfer`,
				input
			)
	};
}
