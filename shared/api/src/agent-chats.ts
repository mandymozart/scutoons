// The Ask panel chats (server `/api/agent-chats`, CUTOVER 10.28): each user's own chats with the agent, per app
// (`creator`, `embassy`, `admin` – the role of that app is checked) – what the panel shows (`turns`) and the model
// conversation (`messages`, only with `messages: true`; the Ask service reads it to continue a chat). Another user's
// chat, or one of another app, answers 404.
import type { ApiClient } from './client.ts';

/** One step of an agent turn: a tool call, and the draft it saved */
export interface AgentStep {
	tool: string;
	target: string;
	error?: string;
	key?: string;
	version?: number;
	/** the page in the app the step is about (app-relative, e.g. the Embassy's `/orgs/<id>`) */
	link?: string;
	/** the step changed something (a log entry, a lead) */
	changed?: boolean;
	/** app-specific result the panel renders (e.g. the Embassy's mail draft) */
	data?: Record<string, unknown>;
}

/** What the panel shows: the creator's message or the agent's answer with its steps */
export interface AgentTurn {
	who: 'you' | 'agent';
	text: string;
	steps: AgentStep[];
	error?: string;
}

export interface AgentChatSummary {
	id: string;
	title: string;
	page: string | null;
	created_at: string;
	updated_at: string;
	/** number of turns */
	turns: number;
}

export interface AgentChat {
	id: string;
	title: string;
	page: string | null;
	created_at: string;
	updated_at: string;
	turns: AgentTurn[];
	app?: AgentApp;
	messages?: unknown[];
}

export type AgentChatsApi = ReturnType<typeof createAgentChatsApi>;

export type AgentApp = 'creator' | 'embassy' | 'admin';

export function createAgentChatsApi(client: ApiClient, app: AgentApp = 'creator') {
	const at = (id: string) => `/agent-chats/${encodeURIComponent(id)}`;
	return {
		list: (limit = 50) => client.get<{ status: string; chats: AgentChatSummary[] }>(`/agent-chats?limit=${limit}&app=${app}`),
		get: (id: string, messages = false) => client.get<{ status: string; chat: AgentChat }>(`${at(id)}${messages ? '?messages=1' : ''}`),
		put: (id: string, chat: { title?: string; page?: string | null; turns: AgentTurn[]; messages: unknown[]; app?: AgentApp }) =>
			client.put<{ status: string; chat: { id: string; updated_at: string } }>(at(id), chat),
		remove: (id: string) => client.delete<{ status: string }>(at(id))
	};
}
