// The creator's Ask panel chats (server `/api/agent-chats`, CUTOVER 10.28): each creator's own chats with the agent –
// what the panel shows (`turns`) and the model conversation (`messages`, only with `messages: true`; the Ask service
// reads it to continue a chat). Role creator; another creator's chat answers 404.
import type { ApiClient } from './client.ts';

/** One step of an agent turn: a tool call, and the draft it saved */
export interface AgentStep {
	tool: string;
	target: string;
	error?: string;
	key?: string;
	version?: number;
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
	messages?: unknown[];
}

export type AgentChatsApi = ReturnType<typeof createAgentChatsApi>;

export function createAgentChatsApi(client: ApiClient) {
	const at = (id: string) => `/agent-chats/${encodeURIComponent(id)}`;
	return {
		list: (limit = 50) => client.get<{ status: string; chats: AgentChatSummary[] }>(`/agent-chats?limit=${limit}`),
		get: (id: string, messages = false) => client.get<{ status: string; chat: AgentChat }>(`${at(id)}${messages ? '?messages=1' : ''}`),
		put: (id: string, chat: { title?: string; page?: string | null; turns: AgentTurn[]; messages: unknown[] }) =>
			client.put<{ status: string; chat: { id: string; updated_at: string } }>(at(id), chat),
		remove: (id: string) => client.delete<{ status: string }>(at(id))
	};
}
