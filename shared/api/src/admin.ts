// Admin API (agents/ITEMS.md → "Admin API"): `/admin/*`, session + role `admin`. Framework-agnostic like the rest
// of the package; the app keeps list state in its URLs and passes the query through.
import type { ApiClient } from './client.ts';
import type { FeaturedProduct } from '@scutoons/content';

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
	/** review before publish: per toon (`*` = every toon) reviewer / needs_review (docs/API.md "Reviews") */
	review_rules?: AdminReviewRule[];
}

export interface AdminReviewRule {
	toon: string;
	/** physical: publishes physical catalog changes of that toon and decides their submits (agents/CATALOG-IN-CREATOR.md) */
	rule: 'reviewer' | 'needs_review' | 'physical';
	user: { id: string; email: string };
	created_at: string | null;
	created_by: string | null;
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
export interface AdminFeatured {
	status: 'success';
	/** the stored list (empty = nothing featured) */
	items: FeaturedProduct[];
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

/** One entry of a PHP error log (`GET /admin/server-errors`, BACKLOG 10.11) – messages only, masked */
export interface AdminServerError {
	/** local time of the server, `YYYY-MM-DDTHH:MM:SS` (null: a line without PHP's date) */
	at: string | null;
	day: string | null;
	level: 'fatal' | 'error' | 'warning' | 'notice';
	message: string;
	/** the lines after it (a stack trace), '' when none */
	detail: string;
}
export interface AdminServerErrors {
	status: string;
	source: 'api' | 'qr';
	sources: { id: 'api' | 'qr'; configured: boolean }[];
	exists: boolean;
	/** the log file's size in bytes; only its last 512 KB are read (`truncated`) */
	size: number;
	truncated: boolean;
	/** the days with entries, newest first */
	days: string[];
	entries: AdminServerError[];
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

// ── Newsletter (agents/NEWSLETTER.md; /admin/newsletter/*) ──────────────────────────────────────────────

export type NewsletterLocale = 'de' | 'en';
export type NewsletterStatus = 'draft' | 'sending' | 'paused' | 'done';

/** A newsletter address that is no account (table mail_contacts) */
export interface NewsletterContact {
	id: number;
	email: string;
	name: string | null;
	/** null = not set yet (a newsletter can't start while a recipient has none) */
	locale: NewsletterLocale | null;
	/** qr-claim (the old QR form) | event:<where, when> | … */
	source: string;
	consented_at: string | null;
	created_at: string;
	created_by: string | null;
	/** has an account (mailed as the member) */
	member: boolean;
	/** unsubscribe | bounce | manual, null = not suppressed */
	suppressed: string | null;
}

export interface NewsletterSource {
	source: string;
	count: number;
	missing_locale: number;
}

/** members_opted_in | members_all | contacts | contacts:<source> */
export interface NewsletterAudience {
	id: string;
	/** internal = our own addresses (@scutoons.com): left out unless picked too */
	kind: 'members' | 'contacts' | 'internal';
	source: string | null;
	count: number;
	missing_locale: number;
}

export type NewsletterImportStatus = 'new' | 'exists' | 'member' | 'suppressed' | 'duplicate' | 'invalid';

export interface NewsletterImportResult {
	status: 'success';
	rows: { line: number; email: string | null; name: string | null; locale: NewsletterLocale | null; status: NewsletterImportStatus }[];
	counts: Record<NewsletterImportStatus, number>;
	added: number;
}

export interface NewsletterSuppression {
	email: string;
	reason: 'unsubscribe' | 'bounce' | 'manual';
	detail: string | null;
	created_at: string;
}

export interface NewsletterProgress {
	queued: number;
	sent: number;
	failed: number;
	skipped: number;
	total: number;
}

export interface Newsletter {
	id: number;
	title: string;
	audiences: string[];
	subject_de: string;
	subject_en: string;
	body_de: string;
	body_en: string;
	status: NewsletterStatus;
	/** why the last batch stopped (SMTP login / connection) */
	last_error: string | null;
	created_at: string;
	created_by: string | null;
	updated_at: string;
	updated_by: string | null;
	started_at: string | null;
	finished_at: string | null;
	progress: NewsletterProgress;
}

export type NewsletterInput = Partial<Pick<Newsletter, 'title' | 'audiences' | 'subject_de' | 'subject_en' | 'body_de' | 'body_en'>>;

export interface NewsletterRecipient {
	id: number;
	email: string;
	locale: NewsletterLocale;
	name: string | null;
	source: 'member' | 'contact';
	status: 'queued' | 'sent' | 'failed' | 'skipped';
	attempts: number;
	next_attempt_at: string | null;
	sent_at: string | null;
	error: string | null;
}

/** One batch (NewsletterSender::batch) */
export interface NewsletterBatchResult {
	status: 'idle' | 'sent' | 'limit' | 'busy' | 'error';
	sent: number;
	failed: number;
	retry: number;
	skipped: number;
	queued: number;
	per_hour: number;
	sent_last_hour: number;
	error: string | null;
}

export interface NewsletterState {
	status: 'success';
	transport: string;
	/** NEWSLETTER_CRON_KEY is set (the host's cron can send) */
	cron: boolean;
	per_hour: number;
	sent_last_hour: number;
	queued: number;
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

/**
 * Changes an admin's own AI proposes (tools/mcp admin.ts `propose_changes`, agents/MCP.md "Draft → link"): a stored
 * proposal (`/admin/proposals`), applied only when the admin presses Apply on its page – by the admin app, with the
 * admin's session. `before` values are what the AI read; Apply refuses a catalog entry / the calendar when it changed
 * since (`base`).
 */
export interface AdminChangeField {
	/** dot path in the entry's data, e.g. `icon`, `name.en`, `variants.0.icon` */
	path: string;
	before: unknown;
	after: unknown;
}

export type AdminChange =
	| { type: 'catalog'; kind: CatalogKind; id: string; label: string; base: string; fields: AdminChangeField[] }
	| { type: 'calendar'; action: 'add'; event: Record<string, unknown> }
	| { type: 'calendar'; action: 'update'; index: number; label: string; fields: AdminChangeField[] }
	| { type: 'asset'; path: string; source: string | null; usages: number };

export type ProposalStatus = 'open' | 'applied' | 'dismissed';

/** A stored proposal (server Domain\Admin\Proposals) */
export interface AdminProposal {
	id: string;
	/** one line: what the changes do */
	title: string;
	changes: AdminChange[];
	/** the calendar's draft version the calendar changes were made on (null = no calendar yet / no calendar change) */
	calendar_base: number | null;
	/** agent = an admin's AI, person = by hand */
	source: 'agent' | 'person';
	status: ProposalStatus;
	created_by: string;
	/** the author's email */
	author: string;
	created_at: string;
	/** who applied / dismissed it (email), '' while open */
	decided_by: string;
	decided_at: string | null;
}

/** A proposal in the list: without its changes, `count` = how many */
export type AdminProposalRow = Omit<AdminProposal, 'changes'> & { count: number };

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
		/** review rules (toon id or `*`, reviewer | needs_review) */
		reviewRules: () => api.get<{ status: string; items: AdminReviewRule[] }>('/admin/review-rules'),
		setReviewRule: (id: string, toon: string, rule: AdminReviewRule['rule'], on: boolean) => {
			const path = `/admin/users/${seg(id)}/review-rules/${seg(toon)}/${rule}`;
			type Result = { status: string; changed: boolean; review_rules: AdminReviewRule[] };
			return on ? api.put<Result>(path) : api.delete<Result>(path);
		},
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

		// the catalog in the database (CUTOVER 10.29): list / get / put (create 201 or update) / delete / reorder per kind,
		// every write validated (422 invalid_catalog { problems }) and recorded in the history (undo)
		catalogList: <T = Record<string, any>>(kind: CatalogKind) =>
			api.get<{ status: 'success'; kind: CatalogKind; items: CatalogEntry<T>[] }>(`/admin/catalog/${seg(kind)}`),
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

		/** the Ask panels' AI usage (from / to: UTC days YYYY-MM-DD, default the last 30 days) */
		// proposals of an admin's own AI (agents/MCP.md "Draft → link")
		proposals: (status?: ProposalStatus) => api.get<{ status: 'success'; items: AdminProposalRow[] }>(`/admin/proposals${toQuery({ status })}`),
		proposal: (id: string) => api.get<{ status: 'success'; proposal: AdminProposal; url: string }>(`/admin/proposals/${seg(id)}`),
		createProposal: (input: { title: string; changes: AdminChange[]; calendar_base?: number | null; source?: 'agent' | 'person' }) =>
			api.post<{ status: 'success'; proposal: AdminProposal; url: string }>('/admin/proposals', input),
		decideProposal: (id: string, status: Exclude<ProposalStatus, 'open'>) =>
			api.put<{ status: 'success'; proposal: AdminProposal }>(`/admin/proposals/${seg(id)}`, { status }),

		featured: () => api.get<AdminFeatured>('/admin/featured'),
		/** the new ordered list, or null = back to the default */
		saveFeatured: (items: FeaturedProduct[]) => api.put<AdminFeatured>('/admin/featured', { items }),

		emailTemplates: () => api.get<AdminEmailTemplates>('/admin/emails/templates'),
		/** a template rendered with its sample vars */
		emailPreview: (id: string, locale: string) =>
			api.get<AdminEmailPreview>(`/admin/emails/templates/${seg(id)}/preview${toQuery({ locale })}`),
		/** the sent-mail log: filters template, status (sent|failed), recipient, from, to; q = part of the address */
		listEmails: (params: ListParams = {}) => api.get<ListResult<AdminEmailLogRow>>(`/admin/emails/log${toQuery(params)}`),
		emailRecipients: () => api.get<AdminEmailRecipients>('/admin/emails/recipients'),
		/** the newest entries of the API's / the qr app's error log, newest first (filters: day, q) */
		serverErrors: (params: { source?: 'api' | 'qr'; day?: string; q?: string; limit?: number } = {}) =>
			api.get<AdminServerErrors>(`/admin/server-errors${toQuery(params)}`),
		/** the new list, or null = back to the default (MAIL_ADMIN_TO) */
		saveEmailRecipients: (items: string[] | null) => api.put<AdminEmailRecipients>('/admin/emails/recipients', { items }),

		// newsletters (agents/NEWSLETTER.md)
		newsletterState: () => api.get<NewsletterState>('/admin/newsletter/status'),
		/** selected = audience ids: also the recipients of that choice together (each address once) */
		newsletterAudiences: (selected?: string[]) =>
			api.get<{ status: 'success'; items: NewsletterAudience[]; selected?: { count: number; missing_locale: number; locales: Record<string, number> } }>(
				`/admin/newsletter/audiences${selected ? toQuery({ selected: selected.join(',') }) : ''}`
			),
		/** filters source, locale (de | en | none); q = part of the address or name */
		newsletterContacts: (params: ListParams = {}) =>
			api.get<ListResult<NewsletterContact> & { sources: NewsletterSource[] }>(`/admin/newsletter/contacts${toQuery(params)}`),
		/** one address per line ("email, name, de"); dry_run = only the preview */
		newsletterImport: (input: { text: string; source: string; consented_at?: string | null; locale?: NewsletterLocale | null; dry_run?: boolean }) =>
			api.post<NewsletterImportResult>('/admin/newsletter/contacts/import', input),
		newsletterUpdateContacts: (ids: number[], patch: { locale?: NewsletterLocale | null; source?: string }) =>
			api.put<{ status: 'success'; changed: number }>('/admin/newsletter/contacts', { ids, ...patch }),
		newsletterUpdateContact: (id: number, patch: { locale?: NewsletterLocale | null; name?: string | null }) =>
			api.put<{ status: 'success'; item: NewsletterContact }>(`/admin/newsletter/contacts/${id}`, patch),
		newsletterDeleteContacts: (ids: number[]) => api.post<{ status: 'success'; deleted: number }>('/admin/newsletter/contacts/delete', { ids }),
		/** filter reason; q = part of the address */
		newsletterSuppressions: (params: ListParams = {}) => api.get<ListResult<NewsletterSuppression>>(`/admin/newsletter/suppressions${toQuery(params)}`),
		newsletterSuppress: (email: string) => api.post<{ status: 'success'; added: boolean }>('/admin/newsletter/suppressions', { email }),
		newsletterUnsuppress: (email: string) => api.post<{ status: 'success'; removed: boolean }>('/admin/newsletter/suppressions/remove', { email }),
		newsletters: () => api.get<{ status: 'success'; items: Newsletter[] }>('/admin/newsletter/campaigns'),
		newsletter: (id: number) => api.get<{ status: 'success'; item: Newsletter }>(`/admin/newsletter/campaigns/${id}`),
		createNewsletter: (input: NewsletterInput & { title: string }) => api.post<{ status: 'success'; item: Newsletter }>('/admin/newsletter/campaigns', input),
		saveNewsletter: (id: number, input: NewsletterInput) => api.put<{ status: 'success'; item: Newsletter }>(`/admin/newsletter/campaigns/${id}`, input),
		deleteNewsletter: (id: number) => api.delete<{ status: 'success' }>(`/admin/newsletter/campaigns/${id}`),
		/** subject / body = unsaved text to preview */
		previewNewsletter: (id: number, locale: NewsletterLocale, draft: { subject?: string; body?: string } = {}) =>
			api.post<{ status: 'success'; locale: NewsletterLocale; subject: string; text: string; html: string }>(`/admin/newsletter/campaigns/${id}/preview`, { locale, ...draft }),
		/** sends the saved text to the signed-in admin */
		testNewsletter: (id: number, locale: NewsletterLocale) =>
			api.post<{ status: 'success'; to: string; locale: NewsletterLocale }>(`/admin/newsletter/campaigns/${id}/test`, { locale }),
		newsletterAction: (id: number, action: 'start' | 'pause' | 'resume') =>
			api.post<{ status: 'success'; item: Newsletter }>(`/admin/newsletter/campaigns/${id}/${action}`),
		/** filters status, locale, source; q = part of the address */
		newsletterRecipients: (id: number, params: ListParams = {}) =>
			api.get<ListResult<NewsletterRecipient>>(`/admin/newsletter/campaigns/${id}/recipients${toQuery(params)}`),
		/** one batch now */
		sendNewsletterBatch: () => api.post<{ status: 'success'; result: NewsletterBatchResult; items: Newsletter[] }>('/admin/newsletter/send'),

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
