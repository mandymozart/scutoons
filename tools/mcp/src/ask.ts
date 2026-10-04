// The Ask service (CUTOVER 10.28 step 2): the creator's "Ask" panel talks to it. One HTTP endpoint, `POST /api/ask`
// (under /api so the single sign-on cookie `scutoons_sid` reaches it in production), streams Server-Sent Events.
// Who asks: the creator's session (Bearer header or the cookie), checked against the API's /me/ – the agent then uses
// that session for every tool, so the API's role checks decide what it may do. The tools are the SCUTOONS MCP server
// (server.ts) connected in-process. The model: Claude via the Anthropic API, paid by the owner; every API call is
// logged (creator, service, model, tokens, cost) – no limits in the app (owner 2026-10-04). Chats are saved in the API
// (`/api/agent-chats`, each creator's own) after every round, so a chat that stops or breaks stays readable and can go
// on (owner 2026-10-04); the memory here is only a cache.
import { appendFileSync, mkdirSync } from 'node:fs';
import { randomUUID } from 'node:crypto';
import { dirname } from 'node:path';
import type { IncomingMessage, ServerResponse } from 'node:http';
import Anthropic from '@anthropic-ai/sdk';
import { ApiError, createAgentChatsApi, createApiClient, type AgentTurn } from '@scutoons/api';
import { Client } from '@modelcontextprotocol/client';
import { InMemoryTransport } from '@modelcontextprotocol/server';
import { isLocalUrl, type Config } from './config.ts';
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

const SYSTEM = [
	'You are the SCUTOONS agent inside the creator app. You help the signed-in creator build their webtoon content with the',
	'SCUTOONS tools: read documents, change them, check them, save drafts, find and upload media. You have exactly the',
	"creator's rights. Work in the creator's language (German or English, as they write).",
	'',
	'Work like this: find the documents first, read the draft (note each locale\'s version id = the base of your save),',
	'change only what was asked, keep every language consistent (structure, media, geometry the same; only texts differ),',
	'save with save_draft (ops preferred over whole documents), fix validation errors before you finish. Everything you',
	'save is a new draft version – the creator reviews and publishes. Never publish unless the creator explicitly asks',
	'for that publish in this chat. When something is unclear or risky, ask before saving. Finish with a short summary of',
	'what you changed (which documents, which languages) – the panel lists your saved drafts as links.',
	'',
	'Answers are shown as simple Markdown in a narrow panel: short paragraphs and "-" lists; bold only for the one thing',
	'that matters (rarely), no tables, no headings for short answers.',
	'',
	'Each user message starts with [Page: … | Text language: … | App language: …]: where the creator is in the app,',
	'which content language they are looking at in the creator (a language code, "source" = each text in its source',
	'language, "all" = every language side by side) and the language of their app. Answer in the app language unless',
	'they write in another one. Content you write goes into every language of the toon (the source language first); when',
	'the text language is one language, that is the one the creator is working on – check it closely.',
	'',
	'# Format rules',
	'',
	...RULES.map((r) => r.text())
].join('\n');

// ── who asks ────────────────────────────────────────────────────────────────────────────────────────────────────────

interface Creator {
	id: string;
	email: string;
	token: string;
}
const sessions = new Map<string, { creator: Creator | null; at: number }>();

function tokenOf(req: IncomingMessage): string | null {
	const bearer = /^Bearer\s+([A-Za-z0-9_-]{20,128})$/.exec(req.headers.authorization?.trim() ?? '')?.[1];
	if (bearer) return bearer;
	const cookie = /(?:^|;\s*)scutoons_sid=([A-Za-z0-9_-]{20,128})/.exec(req.headers.cookie ?? '')?.[1];
	return cookie ?? null;
}

/** The creator behind a session token (API /me/, role creator), cached a minute */
async function creatorOf(api: string, token: string): Promise<Creator | null> {
	const hit = sessions.get(token);
	if (hit && Date.now() - hit.at < 60_000) return hit.creator;
	const res = await fetch(`${api}/me/`, { headers: { Authorization: `Bearer ${token}`, Accept: 'application/json' } });
	const body: any = res.ok ? await res.json() : null;
	const user = body?.user;
	const creator = user && Array.isArray(user.roles) && user.roles.includes('creator') ? { id: String(user.id), email: String(user.email ?? ''), token } : null;
	sessions.set(token, { creator, at: Date.now() });
	return creator;
}

// ── chats (saved in the API, cached here) ───────────────────────────────────────────────────────────────────────────

interface Chat {
	owner: string;
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

function chatsApi(cfg: AskConfig, creator: Creator) {
	return createAgentChatsApi(createApiClient({ baseUrl: cfg.api, getToken: () => creator.token }));
}

/** the creator's chat: from the cache, else from the API (to go on with an earlier chat), else a new one */
async function loadChat(cfg: AskConfig, creator: Creator, id: unknown): Promise<[string, Chat]> {
	if (typeof id === 'string' && /^[0-9a-f-]{36}$/i.test(id)) {
		const cached = chats.get(id);
		if (cached?.owner === creator.id) return [id, cached];
		if (storage && !cached) {
			try {
				const { chat } = await chatsApi(cfg, creator).get(id, true);
				const restored: Chat = { owner: creator.id, title: chat.title, page: chat.page ?? '', messages: (chat.messages ?? []) as Param[], turns: chat.turns ?? [], at: Date.now() };
				chats.set(id, restored);
				return [id, restored];
			} catch (error) {
				if (!(error instanceof ApiError && error.status === 404)) console.error('ask: loading a chat failed', error);
			}
		}
	}
	const chat: Chat = { owner: creator.id, title: '', page: '', messages: [], turns: [], at: Date.now() };
	const fresh = randomUUID();
	chats.set(fresh, chat);
	return [fresh, chat];
}

async function saveChat(cfg: AskConfig, creator: Creator, id: string, chat: Chat) {
	if (!storage) return;
	try {
		await chatsApi(cfg, creator).put(id, { title: chat.title, page: chat.page || null, turns: chat.turns, messages: chat.messages });
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
 * One AI call into the usage log: the API's `agent_usage` (POST /api/agent-usage with the creator's session and the
 * service key – the admin overview reads it), else (no key, no table, API down) a JSON line in the service's own file.
 */
async function logUsage(cfg: AskConfig, creator: Creator, usage: Usage) {
	if (cfg.serviceKey) {
		try {
			const res = await fetch(`${cfg.api}/agent-usage`, {
				method: 'POST',
				headers: { Authorization: `Bearer ${creator.token}`, 'X-Service-Key': cfg.serviceKey, 'Content-Type': 'application/json', Accept: 'application/json' },
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
		appendFileSync(cfg.usageLog, JSON.stringify({ at: new Date().toISOString(), creator: creator.id, email: creator.email, ...usage }) + '\n');
	} catch (error) {
		console.error('ask: usage log failed', error);
	}
}

// ── steps shown in the panel ────────────────────────────────────────────────────────────────────────────────────────

/** What a tool call did, for the panel: a line, and the saved draft (key + version) when there is one */
function stepOf(name: string, input: any, text: string, isError: boolean) {
	let result: any = null;
	try {
		result = JSON.parse(text);
	} catch {
		// plain text result
	}
	const target = input?.key ?? input?.path ?? (input?.toon ? `${input.toon}${input.chapter !== undefined ? ` · ${input.chapter}` : ''}` : '');
	const step: { tool: string; target: string; error?: string; key?: string; version?: number } = { tool: name, target: String(target ?? '') };
	if (isError) step.error = text.slice(0, 300);
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

	const token = tokenOf(req);
	const creator = token ? await creatorOf(cfg.api, token).catch(() => null) : null;
	if (!creator) return void res.writeHead(401, { 'Content-Type': 'application/json' }).end('{"status":"error","error":"unauthorized","message":"Sign in as a creator."}');

	let body: any;
	try {
		body = await readJson(req);
	} catch {
		return void res.writeHead(400).end();
	}
	const text = typeof body.text === 'string' ? body.text.trim() : '';
	if (!text) return void res.writeHead(400).end();

	sweep();
	const [chatId, chat] = await loadChat(cfg, creator, body.chat);
	chat.at = Date.now();
	const page = typeof body.context === 'string' ? body.context.slice(0, 300) : '';
	const short = (v: unknown) => (typeof v === 'string' ? v.slice(0, 20) : '');
	const textLanguage = short(body.textLanguage) || 'source';
	const appLanguage = short(body.appLanguage) || 'en';
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

	// the creator's own tools: the MCP server with their session, connected in-process
	const toolConfig: Config = { api: cfg.api, token: creator.token, tokenSource: 'env', local: isLocalUrl(cfg.api), publish: cfg.publish };
	const [clientSide, serverSide] = InMemoryTransport.createLinkedPair();
	const mcp = createScutoonsServer(toolConfig);
	await mcp.connect(serverSide);
	const client = new Client({ name: 'scutoons-ask', version: '0.1.0' });
	await client.connect(clientSide);
	const tools: Anthropic.Beta.BetaTool[] = (await client.listTools()).tools.map((t) => ({
		name: t.name,
		description: t.description ?? '',
		input_schema: t.inputSchema as Anthropic.Beta.BetaTool.InputSchema,
		eager_input_streaming: true
	}));

	chat.messages.push({ role: 'user', content: `[Page: ${page || '–'} | Text language: ${textLanguage} | App language: ${appLanguage}]\n${text}` });
	await saveChat(cfg, creator, chatId, chat);
	const total = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, cost: 0 };
	let jsonRetries = 0;

	try {
		for (let round = 0; round < MAX_ROUNDS && !closed; round++) {
			const stream = cfg.anthropic.beta.messages.stream({
				model: cfg.model,
				max_tokens: 64000,
				system: [{ type: 'text', text: SYSTEM, cache_control: { type: 'ephemeral' } }],
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
			await logUsage(cfg, creator, {
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
				const r: any = await client.callTool({ name: use.name, arguments: (use.input ?? {}) as Record<string, unknown> }).catch((error) => ({ isError: true, content: [{ type: 'text', text: String(error?.message ?? error) }] }));
				const out = (r.content ?? []).map((c: any) => c.text ?? '').join('');
				results.push({ type: 'tool_result', tool_use_id: use.id, content: out, is_error: !!r.isError });
				send('step', stepOf(use.name, use.input, out, !!r.isError));
			}
			chat.messages.push({ role: 'user', content: results });
			await saveChat(cfg, creator, chatId, chat);
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
		await client.close().catch(() => {});
		await mcp.close().catch(() => {});
		await saveChat(cfg, creator, chatId, chat);
	}
	if (!closed) {
		send('done', { usage: { ...total, cost: Number(total.cost.toFixed(4)) } });
		res.end();
	}
}
