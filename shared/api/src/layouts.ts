// A member's saved creator workspace layouts (server docs/API.md "Workspace layouts", #827). The layout is the creator's
// floating panels' own JSON { places, tabs, stage }: the server checks only that outer shape and its size, the creator's
// readers the inside – so it stays `unknown` here.
import type { ApiClient } from './client.ts';

export interface SavedLayout {
	workspace: string;
	/** an own workspace: the workspace it is made from; null for the creator's own workspaces */
	kind: string | null;
	/** an own workspace's name; null for the creator's own workspaces */
	name: string | null;
	layout: unknown;
	saved_at: string | null;
}

export interface CraftLayout {
	workspace: string;
	craft: string;
	layout: unknown;
}

export interface MemberLayouts {
	/** false: the server has no layouts yet (before its migration) */
	available: boolean;
	layouts: SavedLayout[];
	/** per workspace the default of the member's first craft that has one */
	defaults: CraftLayout[];
}

export function createLayoutsApi(api: ApiClient) {
	return {
		mine: () => api.get<{ status: string } & MemberLayouts>('/me/layouts'),
		/** a new own workspace sends `name` and `kind` (the workspace it is made from) */
		save: (workspace: string, body: { layout: unknown; name?: string; kind?: string }) => api.put<{ status: string; layout: SavedLayout }>(`/me/layouts/${encodeURIComponent(workspace)}`, body),
		/** back to the default; an own workspace is gone */
		remove: (workspace: string) => api.delete<{ status: string; deleted: boolean }>(`/me/layouts/${encodeURIComponent(workspace)}`)
	};
}

export type LayoutsApi = ReturnType<typeof createLayoutsApi>;
