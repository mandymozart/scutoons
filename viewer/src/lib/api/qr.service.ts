// Typed client for the qr.scutoons.com API (agents/ITEMS.md → HTTP API contract)
import { api } from './client.ts';
import type { Avatar, Catalog, CodeInfo, Edition, Entry, GrantSource, Me, User } from '$lib/models/items.ts';

export interface VerifyResult {
	status: string;
	session_token: string;
	user: User;
	claimed?: string[];
}

/** `/access/`: what a code grants to whoever holds it (device viewing access), never who owns it */
export interface AccessResult {
	status: string;
	code: string;
	type: string;
	expires_at: string | null;
	permissions: string[];
	claimable: boolean;
	claimed: boolean;
	claimed_by_me?: boolean;
	/** server-signed access token (for a future server-side content gate) */
	access_token?: string;
}

export interface InventoryResult {
	status: string;
	inventory: Entry[];
}

export interface ProgressResult {
	status?: 'success' | 'stale';
	state: any;
	/** ms since 1970 */
	updated_at: number | null;
}

export const authRequest = (email: string, code?: string | null, locale?: string) =>
	api.post<{ status: string }>('/auth/request/', { email, code: code || undefined, locale });

export const authVerifyOtp = (email: string, otp: string) =>
	api.post<VerifyResult>('/auth/verify/', { email, otp });

export const authVerifyToken = (token: string) => api.post<VerifyResult>('/auth/verify/', { token });

/** Device access check for a code (no login needed; sends the session if there is one). 404 / 410 expired / 429 */
export const checkAccess = (code: string) => api.post<AccessResult>('/access/', { code });

export const authLogout = () => api.post('/auth/logout/');

export const fetchMe = () => api.get<Me>('/me/');

export const fetchCatalog = () => api.get<Catalog>('/items/', { auth: false });

export const fetchEditions = () => api.get<Edition[]>('/editions/', { auth: false });

export const fetchCode = (code: string) =>
	api.get<CodeInfo>(`/code/?c=${encodeURIComponent(code)}`);

export const claimCode = (code: string, item_id?: string | null, variant?: string | null) =>
	api.post<InventoryResult>('/inventory/claim/', {
		code,
		item_id: item_id || undefined,
		variant: variant || undefined
	});

export const craftRecipe = (recipe_id: string) =>
	api.post<InventoryResult>('/inventory/craft/', { recipe_id });

export const grantItem = (item_id: string, source: GrantSource, ref?: string) =>
	api.post<InventoryResult>('/inventory/grant/', { item_id, source, ref });

export const importItems = (items: { item_id: string; qty: number }[]) =>
	api.post<InventoryResult>('/inventory/import/', { items });

export const putAvatar = (slots: Avatar['slots']) =>
	api.put<{ status: string; avatar: Avatar }>('/avatar/', { slots });

export const fetchProgress = (toonId: string) =>
	api.get<ProgressResult>(`/progress/${encodeURIComponent(toonId)}/`);

export const putProgress = (toonId: string, state: any, updated_at: number) =>
	api.put<ProgressResult>(`/progress/${encodeURIComponent(toonId)}/`, { state, updated_at });
