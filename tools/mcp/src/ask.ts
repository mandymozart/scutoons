// The Ask service (CUTOVER 10.28 step 2): the "Ask" panel (@scutoons/ask) of the creator, the embassy and the admin
// talks to it. One HTTP endpoint, `POST /api/ask` (under /api so the single sign-on cookie `scutoons_sid` reaches it
// in production), streams Server-Sent Events. The request names its app; the app decides the role it needs (checked
// against the API's /me/), the system prompt and the tools (APPS): creator → the SCUTOONS content tools (server.ts),
// embassy → the partner network (embassy.ts: read, log, next dates, mail DRAFTS – people send), admin → the content
// tools that only read + the admin API, read and change PROPOSALS (admin.ts: a card the admin applies). The agent
// uses the person's session for every tool, so the API's role checks decide what it may do; the tools are MCP servers
// connected in-process. The model: Claude via the Anthropic API, paid by the owner; every API call is logged (user, app, service, model, tokens, cost) – no limits in
// the app (owner 2026-10-04). Chats are saved in the API (`/api/agent-chats`, each person's own, per app) after every
// round, so a chat that stops or breaks stays readable and can go on (owner 2026-10-04); the memory here is only a
// cache.
import { appendFileSync, mkdirSync } from 'node:fs';
import { randomUUID } from 'node:crypto';
import { dirname } from 'node:path';
import type { IncomingMessage, ServerResponse } from 'node:http';
import Anthropic from '@anthropic-ai/sdk';
import { ApiError, createAgentChatsApi, createApiClient, type AgentApp, type AgentStep, type AgentTurn } from '@scutoons/api';
import { Client } from '@modelcontextprotocol/client';
import { InMemoryTransport, type McpServer } from '@modelcontextprotocol/server';
import { createAdminServer } from './admin.ts';
import { isLocalUrl, type Config } from './config.ts';
import { createEmbassyServer } from './embassy.ts';
import { RULES } from './resources.ts';
import { createScutoonsServer } from './server.ts';

type Param = Anthropic.Beta.BetaMessageParam;

export interface AskConfig {
	api: string;
	model: string;
	/** browser origins allowed to call (CORS with credentials) */
	origins: string[];
	/** JSON lines, one per Anthropic API call */
	/** fallback when the API can't take the usage log */
	usageLog: string;
	/** AGENT_SERVICE_KEY of the API: lets the service write the usage log there ('' = file only) */
	serviceKey: string;
	publish: boolean;
	anthropic: Anthropic;
}

/** USD per million tokens (cache writes: the 5-minute TTL, 1.25 × input) */
const PRICES: Record<string, { input: number; output: number; cacheRead: number; cacheWrite: number }> = {
	'claude-sonnet-5-5': { input: 2, output: 10, cacheRead: 0.2, cacheWrite: 2.5 },
	'claude-opus-5-5': { input: 4, output: 20, cacheRead: 0.2, cacheWrite: 5 }
};
/** a loop guard against an agent that never ends (not a spending limit) */
const MAX_ROUNDS = 60;
const CHAT_TTL_MS = 2 * 60 * 60 * 1000;

const CREATOR_SYSTEM = [
	'You are the SCUTOONS agent inside the creator app. You help the signed-in creator build their webtoon content with the',
	'SCUTOONS tools: read documents, change them, check them, save drafts, find and upload media. You have exactly the',
	"creator's rights. Work in the creator's language (German or English, as they write).",
	'',
	'Work like this: find the documents first, read the draft (note its version id = the base of your save), change only',
	'what was asked (a chapter / scene / season is one language-neutral document: structure once, texts as locale maps),',
	'save with save_draft (ops preferred over whole documents), fix validation errors before you finish. Everything you',
	'save is a new draft version – the creator reviews and publishes. Never publish unless the creator explicitly asks',
	'for that publish in this chat. When something is unclear or risky, ask before saving. Finish with a short summary of',
	'what you changed (which documents, which languages) – the panel lists your saved drafts as links.',
	'',
	'Each user message starts with [Page: … | Text language: … | App language: … | Today: …]: where the creator is in the app,',
	'which content language they are looking at in the creator (a language code, "source" = each text in its source',
	'language, "all" = every language side by side) and the language of their app. Answer in the app language unless',
	'they write in another one. Content you write goes into every language of the toon (the source language first); when',
	'the text language is one language, that is the one the creator is working on – check it closely. Today: the date.',
	'',
	'# Format rules',
	'',
	...RULES.map((r) => r.text())
].join('\n');

// agents/EMBASSY.md "Communication strategy" – the agent writes the way the owner wants the network to be spoken to
const EMBASSY_SYSTEM = [
	'You are the SCUTOONS agent inside Embassy, the partner network of Top Toy / SCUTOONS: stages (venues, festivals,',
	'residencies, web radios), vinyl outlets (distributors, record shops), booking agents and press (radio, blogs,',
	'magazines). You help the signed-in liaison or ambassador keep in touch with them: find organisations and people,',
	'read the communication log, log touchpoints they tell you about, set next check dates, and draft mails – for example',
	'about a new release. You have exactly their rights.',
	'',
	'Sending: you never send. draft_mail puts a draft card in front of the person; they edit it and send it from the app',
	'(one-to-one, from hello@scutoons.com; the sent mail is logged and the lead moves on by itself). Never say a mail was',
	'sent. No mass mail: one individual draft per recipient, and only to addresses in read_org recipients – people who',
	'opted out (do not contact) never get mail, whatever is asked. The opt-out line is added when the mail is sent.',
	'',
	'How we write (relatability beats volume – few, specific, personal messages):',
	'- Every first contact is individual: one line that proves we know them (a show they ran, an article, an artist they',
	'  work with – only what the log, the notes or the person told you; never invent facts), what we are in one sentence',
	'  (Top Toy – music and the SCUTOONS webtoon), one concrete ask, one link (from list_releases). Short.',
	"- In the recipient's language (the person's language; else by the country; German or English when unsure – say which).",
	"- Signed as \"Top Toy / <first name>\" of the person you work for (ask for their name if you don't know it).",
	'- Read the log first: never pitch as new what they already got, answer an open thread instead (reply_to_activity_id),',
	'  and a reply means a conversation – no follow-up sequence after a reply.',
	'- Nothing follows up on its own (owner 2026-10-04): you help the person write mails and prepare calls. Follow-ups',
	'  are mostly phone calls: when a mail went out about a week ago with no answer, suggest the call (the number, what',
	'  to say, which mail to refer to) rather than another mail.',
	"- People work at their own pace (contacts per week): my_portion is their week – today's slice first. Help them",
	'  through it; never pile more on them than their pace.',
	'- Contacts who answered before and went quiet are worth a call: search_orgs with quiet (e.g. 90 days) finds them;',
	'  read the log first and suggest a reason to call (a new release, a show in their city, a thank-you).',
	'',
	'Only log what the person tells you happened. Before changing a lead or the log for several organisations, list what',
	'you will do and ask. Keep answers short; the panel shows your tool calls as steps and drafts as cards.',
	'',
	'A draft card may have been sent since you made it: the log (read_org) shows what went out.',
	'',
	'Each user message starts with [Page: … | App language: … | Today: …]: where the person is in the app (on an',
	'organisation page its name and "(organisation <id>)"), the language of their app – answer in it unless they write',
	'in another one – and the date of today.'
].join('\n');

// the admin app's pages (admin/src/lib/navigation.ts) – the agent points to the right one instead of guessing
const ADMIN_SYSTEM = [
	'You are the SCUTOONS agent inside the admin app. You help the signed-in admin look things up and prepare changes:',
	'codes, the activity log, accounts, claims, mails, the catalog, assets, the calendar, settings, what the AI agents',
	"cost, and the content of the toons (read only). You have exactly the admin's rights.",
	'',
	'Changing things: you never change anything yourself. propose_changes puts a card in front of the admin (before →',
	'after); they press Apply and the admin app makes the changes. Never say something was changed – say what the card',
	'will do. Possible: catalog entries (fields of items, recipes, albums, products, editions, toons – one or many; each',
	'applied change lands in the catalog history with undo), calendar entries (saved as a draft, a person publishes it on',
	'the Calendar page), an asset file replaced under its old path (every use shows the new picture). Not possible here',
	'(hard to undo, done by a person on the page): revoking or transferring codes, deciding claims, deleting anything,',
	'granting roles, asset cleanup, new catalog entries – point to the page. Read before you propose (read_catalog,',
	'read_calendar, asset_usages) and propose only what was asked. The card cannot be edited: details you were not told',
	'(a date, a title, a city) you ask for first – only for an obvious test may you fill in placeholders, and say so.',
	'',
	'The admin app (admin.scutoons.com) – point to the page where something is done or seen:',
	'- Dashboard (/): totals and activity per day and per edition (codes, claimed, revoked, signed, scans, sign-ups) and',
	'  the AI usage (cost per app and person).',
	'- Codes (/codes): every code; filters edition, type, status (claimed, unclaimed, revoked, expired, active, signed,',
	"  unsigned), search, CSV. A code's page /codes/<code>: detail, timeline, certificate signature, ownership history,",
	'  revoke, transfer. Generate (/codes/new): one new code. Batches (/batches): print batches of an edition (reserve',
	'  codes, sheets, certificates, ZIP). Claims (/claims): ownership claims – transfer or reject.',
	'- Users (/users): accounts, roles, review rules, unlocked avatar characters.',
	'- Events (/events): the activity LOG of codes (scans, accesses, claims, logins, sign-ups, revokes).',
	'- Assets (/assets): the media library – search, where a file is used, replace a file in place, rename, relink,',
	'  cleanup, storage.',
	'- Catalog (/catalog/<kind>): items, recipes, albums, products, editions, toons, permissions, code types, meta – the',
	'  catalog is edited HERE (not in the creator), each kind with History + undo; /catalog/items?open=<id> opens one.',
	'  Items / recipes have "Import images …": drop files named <item id>.png or <item id>_<variant>.png – they become',
	"  WebP files and the items' icons (the fastest way to give many items new images from files on the computer).",
	'- Calendar (/calendar): the public events list (scutoons.com/events: gigs, shows, online events) – the content',
	'  document `events`; Save publishes it.',
	'- Homepage (/homepage): the editions featured on the landing page. Settings (/settings): rate limits of QR scans',
	'  and code lookups. Emails (/emails): the sent-mail log, template previews, admin recipients.',
	"- Other apps: the creator (creator.scutoons.com) edits the toons' content (chapters, script, cast, season pages,",
	'  covers); the Embassy (embassy.scutoons.com) is the partner network.',
	'',
	'Words that mean several things – before you look anything up or propose a change, ask back in one short question',
	'(name the meanings) unless the page or the chat already makes it clear: "event" = a calendar entry',
	'(Calendar), an entry of the activity log (Events) or a story event in a chapter script (creator); "edition" = a',
	"catalog edition (the codes belong to it) or a printed batch; \"image of an item\" = the item's icon in the catalog",
	'or the file itself (Assets). Seasons: editions belong to a toon, not to a season – for a season name the toon',
	'(list_toons shows the seasons) and say which editions you counted. "Signed" codes = a certificate signature is',
	'stored.',
	'',
	'Each user message starts with [Page: … | App language: … | Today: …]: where the admin is, the language of their',
	'app – answer in it unless they write in another one – and the date of today.'
].join('\n');

// the panel renders simple Markdown (@scutoons/ask chat-markdown.ts) in a narrow column – every app's agent keeps it sparse
const FORMAT = [
	'Answers are shown as simple Markdown in a narrow panel: short paragraphs and "-" lists; bold only for the one thing',
	'that matters (rarely), no tables, no headings for short answers.'
].join('\n');

/** What each app's panel gets: who may ask, the system prompt, the tools (`readOnly`: only tools that only read) */
const APPS: Record<AgentApp, { roles: string[]; system: string; servers: (config: Config) => McpServer[]; readOnly?: boolean }> = {
	creator: { roles: ['creator'], system: `${CREATOR_SYSTEM}\n\n${FORMAT}`, servers: (c) => [createScutoonsServer(c)] },
	embassy: { roles: ['liaison', 'ambassador', 'admin'], system: `${EMBASSY_SYSTEM}\n\n${FORMAT}`, servers: (c) => [createEmbassyServer(c)] },
	admin: { roles: ['admin'], system: `${ADMIN_SYSTEM}\n\n${FORMAT}`, servers: (c) => [createScutoonsServer(c), createAdminServer(c)], readOnly: true }
};
const isApp = (app: unknown): app is AgentApp => typeof app === 'string' && Object.hasOwn(APPS, app);

// ── who asks ────────────────────────────────────────────────────────────────────────────────────────────────────────

interface Person {
	id: string;
	email: string;
	token: string;
	roles: string[];
}
const sessions = new Map<string, { person: Person | null; at: number }>();

function tokenOf(req: IncomingMessage): string | null {
	const bearer = /^Bearer\s+([A-Za-z0-9_-]{20,128})$/.exec(req.headers.authorization?.trim() ?? '')?.[1];
	if (bearer) return bearer;
	const cookie = /(?:^|;\s*)scutoons_sid=([A-Za-z0-9_-]{20,128})/.exec(req.headers.cookie ?? '')?.[1];
	return cookie ?? null;
}

/** The person behind a session token (API /me/) with their roles, cached a minute */
async function personOf(api: string, token: string): Promise<Person | null> {
	const hit = sessions.get(token);
	if (hit && Date.now() - hit.at < 60_000) return hit.person;
	const res = await fetch(`${api}/me/`, { headers: { Authorization: `Bearer ${token}`, Accept: 'application/json' } });
	const body: any = res.ok ? await res.json() : null;
	const user = body?.user;
	const person = user ? { id: String(user.id), email: String(user.email ?? ''), token, roles: Array.isArray(user.roles) ? user.roles.map(String) : [] } : null;
	sessions.set(token, { person, at: Date.now() });
	return person;
}

// ── chats (saved in the API, cached here) ───────────────────────────────────────────────────────────────────────────

interface Chat {
	owner: string;
	app: AgentApp;
	title: string;
	page: string;
	messages: Param[];
	turns: AgentTurn[];
	at: number;
}
const chats = new Map<string, Chat>();
function sweep() {
	for (const [id, c] of chats) if (Date.now() - c.at > CHAT_TTL_MS) chats.delete(id);
}
/** false once the API answered that it has no chat storage (the migration not run yet) – the panel works without */
let storage = true;

function chatsApi(cfg: AskConfig, person: Person, app: AgentApp) {
	return createAgentChatsApi(createApiClient({ baseUrl: cfg.api, getToken: () => person.token }), app);
}

/** the person's chat in this app: from the cache, else from the API (to go on with an earlier chat), else a new one */
async function loadChat(cfg: AskConfig, person: Person, app: AgentApp, id: unknown): Promise<[string, Chat]> {
	if (typeof id === 'string' && /^[0-9a-f-]{36}$/i.test(id)) {
		const cached = chats.get(id);
		if (cached?.owner === person.id && cached.app === app) return [id, cached];
		if (storage && !cached) {
			try {
				const { chat } = await chatsApi(cfg, person, app).get(id, true);
				const restored: Chat = { owner: person.id, app, title: chat.title, page: chat.page ?? '', messages: (chat.messages ?? []) as Param[], turns: chat.turns ?? [], at: Date.now() };
				chats.set(id, restored);
				return [id, restored];
			} catch (error) {
				if (!(error instanceof ApiError && error.status === 404)) console.error('ask: loading a chat failed', error);
			}
		}
	}
	const chat: Chat = { owner: person.id, app, title: '', page: '', messages: [], turns: [], at: Date.now() };
	const fresh = randomUUID();
	chats.set(fresh, chat);
	return [fresh, chat];
}

async function saveChat(cfg: AskConfig, person: Person, id: string, chat: Chat) {
	if (!storage) return;
	try {
		await chatsApi(cfg, person, chat.app).put(id, { app: chat.app, title: chat.title, page: chat.page || null, turns: chat.turns, messages: chat.messages });
	} catch (error) {
		// 503: the table isn't there yet (migration 2026-10-04-agent-chats) – keep chatting without history
		if (error instanceof ApiError && (error.status === 503 || error.status === 404)) storage = false;
		console.error('ask: saving the chat failed', error instanceof Error ? error.message : error);
	}
}

// ── usage ───────────────────────────────────────────────────────────────────────────────────────────────────────────

export function costOf(model: string, u: Anthropic.Beta.BetaUsage): number {
	const p = PRICES[model];
	if (!p) return 0;
	const m = 1_000_000;
	return (u.input_tokens * p.input + u.output_tokens * p.output + (u.cache_read_input_tokens ?? 0) * p.cacheRead + (u.cache_creation_input_tokens ?? 0) * p.cacheWrite) / m;
}

interface Usage {
	app: AgentApp;
	service: string;
	model: string;
	chat_id: string;
	input_tokens: number;
	output_tokens: number;
	cache_read_tokens: number;
	cache_write_tokens: number;
	cost_usd: number;
}

/**
 * One AI call into the usage log: the API's `agent_usage` (POST /api/agent-usage with the person's session and the
 * service key – the admin overview reads it), else (no key, no table, API down) a JSON line in the service's own file.
 */
async function logUsage(cfg: AskConfig, person: Person, usage: Usage) {
	if (cfg.serviceKey) {
		try {
			const res = await fetch(`${cfg.api}/agent-usage`, {
				method: 'POST',
				headers: { Authorization: `Bearer ${person.token}`, 'X-Service-Key': cfg.serviceKey, 'Content-Type': 'application/json', Accept: 'application/json' },
				body: JSON.stringify(usage)
			});
			if (res.ok) return;
			console.error(`ask: usage log in the API failed (${res.status}) – written to the file`);
		} catch (error) {
			console.error('ask: usage log in the API failed – written to the file', error instanceof Error ? error.message : error);
		}
	}
	try {
		mkdirSync(dirname(cfg.usageLog), { recursive: true });
		appendFileSync(cfg.usageLog, JSON.stringify({ at: new Date().toISOString(), user: person.id, email: person.email, ...usage }) + '\n');
	} catch (error) {
		console.error('ask: usage log failed', error);
	}
}

// ── steps shown in the panel ────────────────────────────────────────────────────────────────────────────────────────

/**
 * What a tool call did, for the panel: a line, the saved draft (key + version) when there is one; embassy steps link to
 * the organisation (`link`, app-relative), say when they changed something and hand a mail draft to the panel (`data`)
 */
function stepOf(name: string, input: any, text: string, isError: boolean) {
	let result: any = null;
	try {
		result = JSON.parse(text);
	} catch {
		// plain text result
	}
	const target = input?.key ?? input?.path ?? input?.to ?? input?.q ?? (input?.toon ? `${input.toon}${input.chapter !== undefined ? ` · ${input.chapter}` : ''}` : '');
	const step: AgentStep = { tool: name, target: String(target ?? '') };
	const org = input?.org_id ?? (name === 'read_org' ? input?.id : undefined);
	if (typeof org === 'string' && /^[0-9a-f-]{36}$/i.test(org)) step.link = `/orgs/${org}`;
	if (name === 'read_org' && result?.org?.name) step.target = result.org.name;
	if (isError) step.error = text.slice(0, 300);
	else if (name === 'propose_changes' && result?.card) Object.assign(step, { data: result.card, target: result.card.title });
	else if (name === 'draft_mail' && result?.draft) Object.assign(step, { data: result.draft, target: `${result.draft.org} · ${result.draft.to}` });
	else if (name === 'log_activity' || name === 'set_lead') step.changed = true;
	else if (name === 'save_draft' && result?.version) Object.assign(step, { key: input.key, version: result.version });
	else if (name === 'restore_version' && result?.version) Object.assign(step, { key: input.key, version: result.version });
	else if (name === 'publish_entity' && result?.published) Object.assign(step, { key: input.key, version: result.published });
	return step;
}

// ── the endpoint ────────────────────────────────────────────────────────────────────────────────────────────────────

function cors(cfg: AskConfig, req: IncomingMessage, res: ServerResponse) {
	const origin = req.headers.origin;
	if (origin && (cfg.origins.includes(origin) || (isLocalUrl(cfg.api) && isLocalUrl(origin)))) {
		res.setHeader('Access-Control-Allow-Origin', origin);
		res.setHeader('Access-Control-Allow-Credentials', 'true');
		res.setHeader('Access-Control-Allow-Headers', 'Authorization, Content-Type');
		res.setHeader('Access-Control-Allow-Methods', 'POST, OPTIONS');
		res.setHeader('Vary', 'Origin');
	}
}

async function readJson(req: IncomingMessage): Promise<any> {
	let raw = '';
	for await (const chunk of req) {
		raw += chunk;
		if (raw.length > 100_000) throw new Error('request too large');
	}
	return JSON.parse(raw || '{}');
}

export async function handle(cfg: AskConfig, req: IncomingMessage, res: ServerResponse) {
	cors(cfg, req, res);
	const path = (req.url ?? '').split('?')[0].replace(/\/+$/, '');
	if (req.method === 'OPTIONS') return void res.writeHead(204).end();
	if (req.method === 'GET' && path === '/api/ask/health') return void res.writeHead(200, { 'Content-Type': 'application/json' }).end('{"status":"ok"}');
	if (req.method !== 'POST' || path !== '/api/ask') return void res.writeHead(404).end();

	let body: any;
	try {
		body = await readJson(req);
	} catch {
		return void res.writeHead(400).end();
	}
	const text = typeof body.text === 'string' ? body.text.trim() : '';
	// requests without `app` come from creator builds before the shared panel
	const app: AgentApp | null = body.app === undefined ? 'creator' : isApp(body.app) ? body.app : null;
	if (!text || !app) return void res.writeHead(400).end();
	const setup = APPS[app];

	// who asks: their session, and the role this app needs
	const token = tokenOf(req);
	const person = token ? await personOf(cfg.api, token).catch(() => null) : null;
	if (!person || !setup.roles.some((r) => person.roles.includes(r)))
		return void res.writeHead(401, { 'Content-Type': 'application/json' }).end(JSON.stringify({ status: 'error', error: 'unauthorized', message: `Sign in with the role ${setup.roles.join(' / ')}.` }));

	sweep();
	const [chatId, chat] = await loadChat(cfg, person, app, body.chat);
	chat.at = Date.now();
	const page = typeof body.context === 'string' ? body.context.slice(0, 300) : '';
	const short = (v: unknown) => (typeof v === 'string' ? v.slice(0, 20) : '');
	const textLanguage = short(body.textLanguage) || 'source';
	const appLanguage = short(body.appLanguage) || 'en';
	// `ref`: what the page shows, for the agent only (the embassy's "organisation <id>")
	const ref = typeof body.ref === 'string' ? body.ref.slice(0, 100) : '';
	const where = `${page || '–'}${ref ? ` (${ref})` : ''}`;
	// today in the owner's time zone: "next Tuesday" and next check dates need it
	const today = new Intl.DateTimeFormat('en-CA', { timeZone: 'Europe/Vienna', year: 'numeric', month: '2-digit', day: '2-digit', weekday: 'long' }).format(new Date());
	const header =
		app === 'creator'
			? `[Page: ${where} | Text language: ${textLanguage} | App language: ${appLanguage} | Today: ${today}]`
			: `[Page: ${where} | App language: ${appLanguage} | Today: ${today}]`;
	if (!chat.title) chat.title = text.replace(/\s+/g, ' ').slice(0, 120);
	if (!chat.page) chat.page = page;
	// what the panel shows, saved with the chat
	chat.turns.push({ who: 'you', text, steps: [] });
	const turn: AgentTurn = { who: 'agent', text: '', steps: [] };
	chat.turns.push(turn);

	res.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache', Connection: 'keep-alive' });
	// every event also goes into the saved turn (the history shows what happened, also when the panel was closed)
	const send = (event: string, data: any) => {
		if (event === 'text') turn.text += data.delta;
		else if (event === 'step') turn.steps.push(data);
		else if (event === 'error') turn.error = data.message;
		if (!closed) res.write(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`);
	};
	let closed = false;
	res.on('close', () => (closed = true));
	send('chat', { id: chatId });

	// the app's tools: its MCP servers with the person's session, connected in-process (publishing only in the creator)
	const toolConfig: Config = { api: cfg.api, token: person.token, tokenSource: 'env', local: isLocalUrl(cfg.api), publish: app === 'creator' && cfg.publish };
	const servers = setup.servers(toolConfig);
	const clients: Client[] = [];
	const owner = new Map<string, Client>();
	const tools: Anthropic.Beta.BetaTool[] = [];
	for (const mcp of servers) {
		const [clientSide, serverSide] = InMemoryTransport.createLinkedPair();
		await mcp.connect(serverSide);
		const client = new Client({ name: 'scutoons-ask', version: '0.1.0' });
		await client.connect(clientSide);
		clients.push(client);
		for (const t of (await client.listTools()).tools) {
			if (setup.readOnly && !t.annotations?.readOnlyHint) continue;
			owner.set(t.name, client);
			tools.push({ name: t.name, description: t.description ?? '', input_schema: t.inputSchema as Anthropic.Beta.BetaTool.InputSchema, eager_input_streaming: true });
		}
	}

	chat.messages.push({ role: 'user', content: `${header}\n${text}` });
	await saveChat(cfg, person, chatId, chat);
	const total = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, cost: 0 };
	let jsonRetries = 0;

	try {
		for (let round = 0; round < MAX_ROUNDS && !closed; round++) {
			const stream = cfg.anthropic.beta.messages.stream({
				model: cfg.model,
				max_tokens: 64000,
				system: [{ type: 'text', text: setup.system, cache_control: { type: 'ephemeral' } }],
				cache_control: { type: 'ephemeral' },
				tools,
				messages: chat.messages,
				output_config: { effort: 'medium' },
				betas: ['server-side-fallback-2026-07-01'],
				fallbacks: 'default'
			});
			stream.on('text', (delta) => send('text', { delta }));
			const abort = () => stream.abort();
			res.once('close', abort);
			let message: Anthropic.Beta.BetaMessage;
			try {
				message = await stream.finalMessage();
				jsonRetries = 0;
			} catch (error) {
				res.off('close', abort);
				// eager input streaming: a tool input that could not be parsed at all → re-issue the turn (twice at most)
				if (error instanceof Anthropic.APIError || closed || jsonRetries++ >= 2) throw error;
				continue;
			}
			res.off('close', abort);

			const u = message.usage;
			const cost = costOf(message.model, u);
			total.input += u.input_tokens;
			total.output += u.output_tokens;
			total.cacheRead += u.cache_read_input_tokens ?? 0;
			total.cacheWrite += u.cache_creation_input_tokens ?? 0;
			total.cost += cost;
			await logUsage(cfg, person, {
				app,
				service: 'anthropic',
				model: message.model,
				chat_id: chatId,
				input_tokens: u.input_tokens,
				output_tokens: u.output_tokens,
				cache_read_tokens: u.cache_read_input_tokens ?? 0,
				cache_write_tokens: u.cache_creation_input_tokens ?? 0,
				cost_usd: Number(cost.toFixed(6))
			});

			chat.messages.push({ role: 'assistant', content: message.content });
			if (message.stop_reason === 'refusal') {
				send('error', { message: 'The model declined this request.' });
				break;
			}
			if (message.stop_reason === 'pause_turn') continue;
			const uses = message.content.filter((b): b is Anthropic.Beta.BetaToolUseBlock => b.type === 'tool_use');
			if (!uses.length) break;
			if (message.stop_reason === 'max_tokens') {
				send('error', { message: 'The answer was cut off (too long).' });
				break;
			}

			const results: Anthropic.Beta.BetaToolResultBlockParam[] = [];
			for (const use of uses) {
				// the MCP server validates the input against the tool's schema before it runs (an error result otherwise)
				const client = owner.get(use.name);
				const r: any = client
					? await client.callTool({ name: use.name, arguments: (use.input ?? {}) as Record<string, unknown> }).catch((error) => ({ isError: true, content: [{ type: 'text', text: String(error?.message ?? error) }] }))
					: { isError: true, content: [{ type: 'text', text: `unknown tool ${use.name}` }] };
				const out = (r.content ?? []).map((c: any) => c.text ?? '').join('');
				results.push({ type: 'tool_result', tool_use_id: use.id, content: out, is_error: !!r.isError });
				send('step', stepOf(use.name, use.input, out, !!r.isError));
			}
			chat.messages.push({ role: 'user', content: results });
			await saveChat(cfg, person, chatId, chat);
		}
	} catch (error) {
		const message =
			error instanceof Anthropic.RateLimitError
				? 'The AI service is busy – try again in a minute.'
				: error instanceof Anthropic.AuthenticationError
					? 'The AI service key is not valid (server configuration).'
					: error instanceof Anthropic.BadRequestError
						? 'The AI service refused the request (details in the service log – e.g. no credit left on the account).'
						: error instanceof Anthropic.APIError
						? `AI service error ${error.status}`
						: error instanceof Error
							? error.message
							: String(error);
		send('error', { message });
		console.error('ask:', error);
	} finally {
		for (const client of clients) await client.close().catch(() => {});
		for (const mcp of servers) await mcp.close().catch(() => {});
		await saveChat(cfg, person, chatId, chat);
	}
	if (!closed) {
		send('done', { usage: { ...total, cost: Number(total.cost.toFixed(4)) } });
		res.end();
	}
}
