// The admin's own tools for the Ask panel in the admin app (role admin, checked by the API on every call): the admin API
// read – dashboard numbers, codes, the activity log, accounts, claims, the mail log, the catalog, assets, settings,
// the calendar, the AI usage – next to the content tools that only read (ask.ts). Changes are never made here:
// `propose_changes` checks them and hands them to the panel as a card; the admin presses Apply and the admin app makes
// them with the admin's session (catalog entries with history + undo, calendar entries as a draft, an asset file
// replaced in place). Deliberately not offered (hard to undo, stay manual): revoking / transferring codes, deletes,
// role grants, asset cleanup (owner 2026-10-05, CUTOVER 10.28).
import { randomUUID } from 'node:crypto';
import { McpServer } from '@modelcontextprotocol/server';
import * as z from 'zod';
import {
	ApiError,
	createAdminApi,
	createApiClient,
	createAssetsApi,
	createContentApi,
	type AdminChange,
	type AdminChangeField,
	type AdminChangesCard,
	type CatalogKind,
	type ListParams
} from '@scutoons/api';
import type { Config } from './config.ts';
import { apiMessage } from './server.ts';

type Result = { content: { type: 'text'; text: string }[]; isError?: boolean };
const text = (value: unknown): Result => ({ content: [{ type: 'text', text: typeof value === 'string' ? value : JSON.stringify(value, null, 1) }] });
const fail = (message: string): Result => ({ ...text(message), isError: true });
const guard =
	<A>(fn: (args: A) => Promise<Result>) =>
	async (args: A): Promise<Result> => {
		try {
			return await fn(args);
		} catch (error) {
			return fail(apiMessage(error));
		}
	};

const read = { readOnlyHint: true, openWorldHint: false };
const day = z.string().regex(/^\d{4}-\d{2}-\d{2}$/).describe('YYYY-MM-DD');
const page = { page: z.number().int().min(1).optional(), page_size: z.number().int().min(1).max(100).optional().describe('default 25') };

const CODE_STATUSES = ['claimed', 'unclaimed', 'revoked', 'expired', 'active', 'signed', 'unsigned'] as const;
const LOG_EVENTS = ['scan', 'access', 'claim', 'claim_refused', 'login', 'signup', 'revoke', 'unrevoke'] as const;
const CATALOG_READ = ['items', 'recipes', 'albums', 'products', 'editions', 'featured', 'permissions', 'code-types', 'toons', 'meta'] as const;
/** what a changes card may change in the catalog (never permissions / code types: they decide what codes unlock) */
const CATALOG_CHANGE = ['items', 'recipes', 'albums', 'products', 'editions', 'toons'] as const;
const CALENDAR_FIELDS = ['title', 'date', 'city', 'location', 'category', 'link'] as const;
const CALENDAR = 'events';

const query = (params: Record<string, unknown>): ListParams =>
	Object.fromEntries(Object.entries(params).filter(([, v]) => v !== undefined && v !== null && v !== '')) as ListParams;

// ── dot paths in catalog data (`icon`, `name.en`, `variants.0.icon`) ───────────────────────────────────────────────────

const segments = (path: string) => path.split('.').map((s) => (/^\d+$/.test(s) ? Number(s) : s));

export function getPath(data: unknown, path: string): unknown {
	let at: any = data;
	for (const s of segments(path)) {
		if (at === null || typeof at !== 'object') return undefined;
		at = at[s];
	}
	return at;
}

/** sets a value in a copy; the parent must exist (an array index only within the array) – else an error message */
export function setPath<T>(data: T, path: string, value: unknown): T | string {
	const copy = structuredClone(data) as any;
	const parts = segments(path);
	let at = copy;
	for (const [i, s] of parts.entries()) {
		const last = i === parts.length - 1;
		if (at === null || typeof at !== 'object') return `${path}: ${parts.slice(0, i).join('.') || 'the entry'} is not an object`;
		if (Array.isArray(at) && (typeof s !== 'number' || s >= at.length)) return `${path}: no element ${s} in ${parts.slice(0, i).join('.')} (${at.length} elements)`;
		if (last) {
			if (value === null) {
				if (Array.isArray(at)) return `${path}: list elements are not removed here`;
				delete at[s];
			} else at[s] = value;
		} else at = at[s];
	}
	return copy;
}

const same = (a: unknown, b: unknown) => JSON.stringify(a ?? null) === JSON.stringify(b ?? null);
const name = (data: any) => (typeof data?.name === 'string' ? data.name : (data?.name?.en ?? data?.name?.de ?? data?.title?.en ?? data?.title ?? ''));

export function createAdminServer(config: Config): McpServer {
	const client = createApiClient({ baseUrl: config.api, getToken: () => config.token });
	const admin = createAdminApi(client);
	const assets = createAssetsApi(client);
	const content = createContentApi(client);
	const server = new McpServer(
		{ name: 'scutoons-admin', version: '0.2.0' },
		{ instructions: 'SCUTOONS admin: read the admin data; changes only as a proposal (propose_changes) the admin applies.' }
	);

	// ── overview ────────────────────────────────────────────────────────────────────────────────────────────────────

	server.registerTool(
		'dashboard',
		{
			description:
				'The Dashboard numbers: totals (codes, claimed, revoked, signed = a certificate signature is stored; users, opted in, new), the activity of the last `days` (scans, accesses, claims, logins, sign-ups, revokes, unique devices) and the same per edition (name, toon, codes, claimed, revoked, signed). `daily: true` adds the day-by-day counts.',
			inputSchema: z.object({ days: z.number().int().min(1).max(365).optional().describe('default 30'), edition_id: z.string().optional(), daily: z.boolean().optional() }),
			annotations: read
		},
		guard(async ({ days, edition_id, daily }) => {
			const d = await admin.dashboard({ days, edition_id });
			return text(daily ? d : { from: d.from, to: d.to, days: d.days, edition_id: d.edition_id, totals: d.totals, period: d.period, editions: d.editions });
		})
	);

	server.registerTool(
		'count_codes',
		{
			description:
				'How many codes match, per edition: filter by toon (its editions) or edition, status (claimed, unclaimed, revoked, expired, active, signed = certificate signature stored, unsigned) and type. Editions have a toon but no season: a season question means the toon\'s editions – say which editions you counted.',
			inputSchema: z.object({
				toon: z.string().optional().describe('toon id, e.g. catpope'),
				edition_id: z.string().optional().describe('an edition id, or "none" = codes without an edition'),
				status: z.enum(CODE_STATUSES).optional(),
				type: z.string().optional()
			}),
			annotations: read
		},
		guard(async ({ toon, edition_id, status, type }) => {
			const editions = (await admin.catalogList('editions')).items.filter((e) => (!toon || e.data?.toon === toon) && (!edition_id || e.id === edition_id));
			const count = async (edition: string | undefined) => (await admin.listCodes(query({ edition_id: edition, status, type, page_size: 1 }))).pagination.total;
			if (edition_id === 'none') return text({ status: status ?? 'all', type: type ?? 'all', editions: [{ id: 'none', codes: await count('none') }] });
			if (toon && !editions.length) return fail(`No edition belongs to the toon "${toon}" – see read_catalog editions.`);
			const rows = await Promise.all(editions.map(async (e) => ({ id: e.id, name: e.data?.name ?? '', toon: e.data?.toon ?? null, codes: await count(e.id) })));
			const total = toon || edition_id ? rows.reduce((sum, r) => sum + r.codes, 0) : await count(undefined);
			return text({ status: status ?? 'all', type: type ?? 'all', total, editions: rows });
		})
	);

	// ── codes ───────────────────────────────────────────────────────────────────────────────────────────────────────

	server.registerTool(
		'list_codes',
		{
			description: 'Codes (the Codes page): code, type, edition and number, claimed, revoked, signed, scans / accesses / devices, first / last seen. Filters as on the page; `q` = part of the code. Never shows owners.',
			inputSchema: z.object({
				edition_id: z.string().optional(),
				status: z.enum(CODE_STATUSES).optional(),
				type: z.string().optional(),
				q: z.string().optional(),
				sort: z.string().optional().describe('code, type, edition_number, created_at, claimed_at, revoked_at, scans, last_seen …; -field = descending'),
				...page
			}),
			annotations: read
		},
		guard(async (args) => text(await admin.listCodes(query({ page_size: 25, ...args }))))
	);

	server.registerTool(
		'read_code',
		{
			description: 'One code: its detail (type, edition, claimed, revoked, signature, items), its timeline (scans, accesses, claims …, newest first) and its ownership history (accounts as emails – admin only).',
			inputSchema: z.object({ code: z.string().min(1).max(64), events: z.number().int().min(0).max(200).optional().describe('timeline entries, default 30') }),
			annotations: read
		},
		guard(async ({ code, events }) => {
			const [detail, timeline, history] = await Promise.all([
				admin.getCode(code),
				admin.codeEvents(code, events ?? 30),
				admin.codeHistory(code).catch(() => ({ items: [] }))
			]);
			return text({ ...detail, timeline: timeline.items, ownership: history.items });
		})
	);

	server.registerTool(
		'list_events',
		{
			description:
				'The activity LOG (the Events page; not the calendar): scans, accesses, claims, refused claims, logins, sign-ups, revokes – newest first. Filters: code, event (comma list), edition_id, toon, from / to (days).',
			inputSchema: z.object({
				code: z.string().optional(),
				event: z.string().optional().describe(`comma list of ${LOG_EVENTS.join(', ')}`),
				edition_id: z.string().optional(),
				toon: z.string().optional(),
				from: day.optional(),
				to: day.optional(),
				...page
			}),
			annotations: read
		},
		guard(async (args) => text(await admin.listEvents(query({ page_size: 25, ...args }))))
	);

	// ── people ──────────────────────────────────────────────────────────────────────────────────────────────────────

	server.registerTool(
		'list_users',
		{
			description: 'Accounts (the Users page): email, created, roles, signup code, marketing opt-in, number of codes. `q` = part of the email; role (admin, creator, liaison, ambassador …); opt_in 1 / 0.',
			inputSchema: z.object({ q: z.string().optional(), role: z.string().optional(), opt_in: z.enum(['1', '0']).optional(), sort: z.string().optional().describe('created_at, email, codes; -field = descending'), ...page }),
			annotations: read
		},
		guard(async (args) => text(await admin.listUsers(query({ page_size: 25, ...args }))))
	);

	server.registerTool(
		'read_user',
		{
			description: 'One account: roles, locale, opt-in, its codes, unlocked avatar characters, review rules.',
			inputSchema: z.object({ id: z.string().min(1) }),
			annotations: read
		},
		guard(async ({ id }) => text((await admin.getUser(id)).user))
	);

	server.registerTool(
		'list_claims',
		{
			description: 'Ownership claims ("this is mine" on a code registered to someone else; the Claims page): status open / transferred / rejected / withdrawn, code, claimant, owner, statement (short). `open` = how many are open.',
			inputSchema: z.object({ status: z.enum(['open', 'transferred', 'rejected', 'withdrawn']).optional(), code: z.string().optional(), q: z.string().optional(), ...page }),
			annotations: read
		},
		guard(async (args) => text(await admin.listClaims(query({ page_size: 25, ...args }))))
	);

	server.registerTool(
		'read_claim',
		{
			description: 'One claim with its statement, files (names only), the code, the current owner, the certificate check, history and other claims. Deciding (transfer / reject) is done by a person on the claim page.',
			inputSchema: z.object({ id: z.string().regex(/^[0-9a-f-]{36}$/) }),
			annotations: read
		},
		guard(async ({ id }) => text(await admin.getClaim(id)))
	);

	server.registerTool(
		'email_log',
		{
			description: 'The sent-mail log (the Emails page): template, recipient, locale, sent / failed, error, time – never the text. Filters template, status, recipient, from / to (days); `q` = part of the address.',
			inputSchema: z.object({ template: z.string().optional(), status: z.enum(['sent', 'failed']).optional(), recipient: z.string().optional(), from: day.optional(), to: day.optional(), q: z.string().optional(), ...page }),
			annotations: read
		},
		guard(async (args) => text(await admin.listEmails(query({ page_size: 25, ...args }))))
	);

	// ── catalog, calendar, assets, settings ─────────────────────────────────────────────────────────────────────────

	server.registerTool(
		'read_catalog',
		{
			description:
				'The catalog (edited on /catalog/<kind>): items, recipes, albums, products, editions, featured, permissions, code-types, toons, meta. One entry (`id`) or the list in order; `q` = words in the entry (any field); `fields` = only these dot paths (e.g. ["name.en", "icon", "variants"]) to keep long lists short. Image paths (`icon`, `image` …) are CDN paths. Each entry has updated_at / updated_by.',
			inputSchema: z.object({
				kind: z.enum(CATALOG_READ),
				id: z.string().optional(),
				q: z.string().optional(),
				fields: z.array(z.string()).max(20).optional(),
				limit: z.number().int().min(1).max(500).optional().describe('default 100')
			}),
			annotations: read
		},
		guard(async ({ kind, id, q, fields, limit }) => {
			const pick = (data: any) => (fields?.length ? Object.fromEntries(fields.map((f) => [f, getPath(data, f) ?? null])) : data);
			if (id) {
				const { entry } = await admin.catalogGet(kind, id);
				return text({ ...entry, data: pick(entry.data) });
			}
			const words = (q ?? '').toLowerCase().split(/\s+/).filter(Boolean);
			const all = (await admin.catalogList(kind)).items;
			const hits = all.filter((e) => {
				const hay = `${e.id} ${JSON.stringify(e.data)}`.toLowerCase();
				return words.every((w) => hay.includes(w));
			});
			const max = limit ?? 100;
			return text({ kind, total: hits.length, shown: Math.min(max, hits.length), items: hits.slice(0, max).map((e) => ({ id: e.id, updated_at: e.updated_at, updated_by: e.updated_by, data: pick(e.data) })) });
		})
	);

	server.registerTool(
		'catalog_history',
		{
			description: 'Changes of the catalog, newest first (who, when, before / after, undo of). Every change can be undone on its catalog page (History).',
			inputSchema: z.object({ kind: z.enum(CATALOG_READ).optional(), id: z.string().optional(), limit: z.number().int().min(1).max(100).optional().describe('default 20') }),
			annotations: read
		},
		guard(async ({ kind, id, limit }) => text((await admin.catalogHistory({ kind: kind as CatalogKind | undefined, id, limit: limit ?? 20 })).items))
	);

	server.registerTool(
		'read_calendar',
		{
			description:
				'The calendar (the Calendar page; the public events list on scutoons.com/events – gigs, shows, online events): the content document `events`, its newest version (draft or published) with each entry\'s `index` (for propose_changes).',
			inputSchema: z.object({}),
			annotations: read
		},
		guard(async () => {
			try {
				const doc = await content.getDraft<Record<string, unknown>[]>(CALENDAR);
				const list = Array.isArray(doc.content) ? doc.content : [];
				return text({ version: doc.version.id, status: doc.version.status, saved_at: doc.version.created_at, events: list.map((event, index) => ({ index, ...event })) });
			} catch (error) {
				if (error instanceof ApiError && error.status === 404) return text({ version: null, events: [] });
				throw error;
			}
		})
	);

	server.registerTool(
		'asset_usages',
		{
			description:
				'One media file by its CDN path (e.g. "catpope/items/camera.webp"): the asset (id, versions, renditions) and every place that uses it – content documents (draft / published) and catalog entries (`item:camera` …). Replacing the file in place (propose_changes asset) changes the picture in all of them at once.',
			inputSchema: z.object({ path: z.string().min(1) }),
			annotations: read
		},
		guard(async ({ path }) => {
			const found = await assets.lookup(path);
			if (!found.asset) return fail(`No asset has the file ${path} (search_assets finds files).`);
			const a = found.asset;
			return text({
				asset: a.asset,
				current: a.versions.at(-1) ?? null,
				versions: a.versions.length,
				usages: a.usages.map((u) => ({ ref: u.ref, source: u.source, field: u.field, state: u.state, path: u.path }))
			});
		})
	);

	server.registerTool(
		'read_settings',
		{
			description: 'Runtime settings (the Settings page): each key with its value, default, min / max, who changed it when (rate limits of QR scans and code lookups per hour), and the admin mail recipients.',
			inputSchema: z.object({}),
			annotations: read
		},
		guard(async () => {
			const [settings, recipients] = await Promise.all([admin.settings(), admin.emailRecipients().catch(() => null)]);
			return text({ settings: settings.items, mail_admin_recipients: recipients?.items ?? null });
		})
	);

	server.registerTool(
		'agent_usage',
		{
			description: 'What the AI agents cost: calls, tokens and USD per user, app (creator / embassy / admin), service and model, per day or month. Dates YYYY-MM-DD (default: the last 30 days). The Dashboard shows the same.',
			inputSchema: z.object({ from: day.optional(), to: day.optional(), group: z.enum(['day', 'month']).optional() }),
			annotations: read
		},
		guard(async (args) => text(await admin.agentUsage(args)))
	);

	// ── changes: a card the admin applies ───────────────────────────────────────────────────────────────────────────

	const fieldsOf = (data: unknown, set: Record<string, unknown>, where: string): AdminChangeField[] | string => {
		let next = data;
		const fields: AdminChangeField[] = [];
		for (const [path, after] of Object.entries(set)) {
			if (path === 'id' || path.startsWith('id.')) return `${where}: the id is not changed here`;
			const before = getPath(data, path) ?? null;
			const result = setPath(next, path, after);
			if (typeof result === 'string') return `${where}: ${result}`;
			next = result;
			if (!same(before, after)) fields.push({ path, before, after });
		}
		return fields;
	};

	const change = z.union([
		z.object({
			type: z.literal('catalog'),
			kind: z.enum(CATALOG_CHANGE),
			id: z.string().min(1),
			set: z.record(z.string(), z.unknown()).describe('dot path → new value (JSON), e.g. { "icon": "catpope/items/new.webp", "name.en": "…", "variants.0.icon": "…" }; null removes an optional field')
		}),
		z.object({
			type: z.literal('calendar_add'),
			event: z.object({
				title: z.string().min(1),
				date: z.string().regex(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(:\d{2})?$/).describe('local time YYYY-MM-DDTHH:MM'),
				city: z.string().min(1),
				location: z.string().optional(),
				category: z.enum(['irl', 'virtual', 'hybrid']),
				link: z.string().url().optional()
			})
		}),
		z.object({
			type: z.literal('calendar_update'),
			index: z.number().int().min(0).describe('the entry\'s index from read_calendar'),
			set: z.record(z.enum(CALENDAR_FIELDS), z.unknown())
		}),
		z.object({
			type: z.literal('asset'),
			path: z.string().min(1).describe('the file to replace (CDN path) – it keeps its path, every use shows the new picture'),
			source: z.string().optional().describe('an asset path whose file becomes the new one (same file type); left out = the admin picks the file in the card')
		})
	]);

	server.registerTool(
		'propose_changes',
		{
			description:
				'Proposes changes as a card in the panel – nothing is changed by you: the admin reads the card and presses Apply, then the admin app makes the changes. Kinds: `catalog` (fields of an existing entry of items / recipes / albums / products / editions / toons – one card can change many entries; every applied change lands in the catalog history with undo), `calendar_add` / `calendar_update` (the calendar, saved as a DRAFT – a person publishes it on the Calendar page), `asset` (a file replaced under its old path: every use shows the new picture – the fast way to swap the same image in many places). Read first (read_catalog / read_calendar / asset_usages); the card shows before → after. Not possible here: deleting, revoking / transferring codes, roles, asset cleanup.',
			inputSchema: z.object({
				title: z.string().min(1).max(200).describe('one line: what the changes do, in the admin\'s language'),
				changes: z.array(change).min(1).max(100)
			}),
			annotations: read
		},
		guard(async ({ title, changes: wanted }) => {
			const changes: AdminChange[] = [];
			const problems: string[] = [];
			let calendar: { version: number | null; list: Record<string, unknown>[] } | null = null;
			const loadCalendar = async () => {
				if (calendar) return calendar;
				try {
					const doc = await content.getDraft<Record<string, unknown>[]>(CALENDAR);
					calendar = { version: doc.version.id, list: Array.isArray(doc.content) ? doc.content : [] };
				} catch (error) {
					if (!(error instanceof ApiError && error.status === 404)) throw error;
					calendar = { version: null, list: [] };
				}
				return calendar;
			};

			for (const c of wanted) {
				if (c.type === 'catalog') {
					let entry;
					try {
						entry = (await admin.catalogGet(c.kind, c.id)).entry;
					} catch (error) {
						problems.push(error instanceof ApiError && error.status === 404 ? `${c.kind} ${c.id}: no such entry (new entries are made on the catalog page)` : `${c.kind} ${c.id}: ${apiMessage(error)}`);
						continue;
					}
					const fields = fieldsOf(entry.data, c.set, `${c.kind} ${c.id}`);
					if (typeof fields === 'string') problems.push(fields);
					else if (fields.length) changes.push({ type: 'catalog', kind: c.kind, id: c.id, label: String(name(entry.data) || c.id), base: entry.updated_at, fields });
				} else if (c.type === 'calendar_add') {
					await loadCalendar();
					const e = c.event;
					const event: Record<string, unknown> = { title: e.title.trim(), date: e.date.length === 16 ? `${e.date}:00` : e.date, city: e.city.trim(), location: e.location?.trim() ?? '', category: e.category };
					if (e.link) event.link = e.link;
					changes.push({ type: 'calendar', action: 'add', event });
				} else if (c.type === 'calendar_update') {
					const { list } = await loadCalendar();
					const current = list[c.index];
					if (!current) {
						problems.push(`calendar: no entry ${c.index} (read_calendar lists ${list.length})`);
						continue;
					}
					const fields = fieldsOf(current, c.set, `calendar ${c.index}`);
					if (typeof fields === 'string') problems.push(fields);
					else if (fields.length) changes.push({ type: 'calendar', action: 'update', index: c.index, label: `${current.title ?? ''} · ${String(current.date ?? '').slice(0, 10)}`, fields });
				} else {
					const found = await assets.lookup(c.path);
					if (!found.asset) {
						problems.push(`${c.path}: no asset has this file (search_assets)`);
						continue;
					}
					const ext = (p: string) => /\.([a-z0-9]+)$/i.exec(p)?.[1]?.toLowerCase() ?? '';
					if (c.source) {
						const src = await assets.lookup(c.source);
						if (!src.asset) {
							problems.push(`${c.source}: no asset has this file`);
							continue;
						}
						if (ext(c.source) !== ext(c.path)) {
							problems.push(`${c.path}: the new file must be the same type (.${ext(c.path)}), ${c.source} is .${ext(c.source)}`);
							continue;
						}
					}
					changes.push({ type: 'asset', path: c.path, source: c.source ?? null, usages: found.asset.usages.length });
				}
			}
			if (problems.length) return fail(`Nothing proposed – fix these first:\n- ${problems.join('\n- ')}`);
			if (!changes.length) return fail('Nothing to change: every value is already what you proposed.');
			const card: AdminChangesCard = {
				kind: 'admin_changes',
				id: randomUUID(),
				title,
				changes,
				calendar_base: (calendar as { version: number | null } | null)?.version ?? null
			};
			return text({ card, shown: 'The admin sees the changes as a card (before → after) and applies them with Apply – nothing is changed yet. Do not repeat the list in your answer; say in one line what the card does and where to check it afterwards.' });
		})
	);

	return server;
}
