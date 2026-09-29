// Typed client for the qr.scutoons.com API (agents/ITEMS.md → HTTP API contract)
import { api } from './client.ts';
import type { Avatar, Catalog, CodeInfo, Edition, Entry, GrantSource, Me, User } from '@scutoons/content';

export interface VerifyResult {
	status: string;
	session_token: string;
	user: User;
	/** true = the account was just created (signup with a membership or promo code) */
	created?: boolean;
	/** only for claimable codes (a promo code is never owned) */
	claimed?: string[];
	claim_error?: { code: string; error: string; message: string; options?: unknown };
}

/** `/access/`: what a code grants to whoever holds it (device viewing access), never who owns it */
export interface AccessResult {
	status: string;
	code: string;
	type: string;
	expires_at: string | null;
	permissions: string[];
	claimable: boolean;
	/** a new account can be created with this code (membership, promo) */
	signup?: boolean;
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

/**
 * Sends the login mail. New accounts only with a signup code (membership / promo): without an account and without
 * such a code the answer is still `sent`, but the mail only explains how to get an account (no one-time code).
 */
export const authRequest = (
	email: string,
	code?: string | null,
	locale?: string,
	marketingOptIn = false,
	choice?: { item_id?: string | null; variant?: string | null }
) =>
	api.post<{ status: string }>('/auth/request/', {
		email,
		code: code || undefined,
		locale,
		marketing_opt_in: marketingOptIn,
		// the item picked for a code with options (a new account claims it in the same transaction)
		item_id: choice?.item_id || undefined,
		variant: choice?.variant || undefined
	});

/**
 * `marketing_opt_in`: stored for a new account; `true` also opts an existing account in.
 * New account with a product code = account + claim in one step: 409 already_claimed / 422 choose_item → no account.
 * 403 signup_code_required.
 */
export const authVerifyOtp = (email: string, otp: string, marketingOptIn?: boolean) =>
	api.post<VerifyResult>('/auth/verify/', { email, otp, marketing_opt_in: marketingOptIn });

export const authVerifyToken = (token: string) => api.post<VerifyResult>('/auth/verify/', { token });

/** Device access check for a code (no login needed; sends the session if there is one). 404 / 410 expired / 429 */
export const checkAccess = (code: string) => api.post<AccessResult>('/access/', { code });

export const authLogout = () => api.post('/auth/logout/');

export const fetchMe = () => api.get<Me>('/me/');

/** Account settings: `PUT /me/ { marketing_opt_in }` → the updated user */
export const updateMe = (patch: { marketing_opt_in: boolean }) =>
	api.put<{ status: string; user: User }>('/me/', patch);

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
