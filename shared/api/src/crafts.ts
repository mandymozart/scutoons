// Crafts (server docs/API.md "Crafts", #826): what a member does – illustrator, writer, translator … – and so which creator
// workspaces they start with. Not the catalog recipes' crafting. The craft → workspaces map is the server's; clients read it.
import type { ApiClient } from './client.ts';

export interface Craft {
	id: string;
	/** locale map { en, de, … } */
	name: Record<string, string>;
	/** a shared/ui icon name */
	icon: string;
	/** the creator workspace ids it starts with, in order */
	workspaces: string[];
}

export interface MemberCrafts {
	crafts: string[];
	/** those of `crafts` that came with an admin's invite */
	invited: string[];
	/** the workspaces the crafts start with, resolved by the server, in the crafts' order */
	workspaces: string[];
	/** the first-login question is still open */
	ask: boolean;
	/** the member may change them now (the question is open, or an admin) */
	can_edit: boolean;
}

export interface MemberInvite {
	id: string;
	email: string;
	crafts: string[];
	invited_by: string | null;
	created_at: string | null;
	accepted_at: string | null;
	user_id: string | null;
}

export interface MemberCraft {
	id: string;
	source: 'invite' | 'admin' | 'member';
}

/** the craft's name in `locale`, else English, else its id */
export const craftName = (craft: Craft, locale: string): string => craft.name[locale] ?? craft.name.en ?? craft.id;

export function createCraftsApi(api: ApiClient) {
	return {
		list: () => api.get<{ status: string; crafts: Craft[] }>('/crafts'),
		mine: () => api.get<{ status: string } & MemberCrafts>('/me/crafts'),
		/** only while `ask` (first login) or as an admin – else 403 crafts_locked */
		setMine: (crafts: string[]) => api.put<{ status: string } & MemberCrafts>('/me/crafts', { crafts })
	};
}

export type CraftsApi = ReturnType<typeof createCraftsApi>;
