// Typed endpoints of the SCUTOONS API: accounts, access, items, inventory, avatar, progress
// (agents/ITEMS.md → HTTP API contract). `createQrApi(client)` binds them to one configured client.
import type { ApiClient } from './client.ts';
import type { Avatar, AvatarBodyRef, Catalog, CodeInfo, Edition, Entry, GrantSource, Me, User } from '@scutoons/content';

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

export type QrApi = ReturnType<typeof createQrApi>;

export function createQrApi(api: ApiClient) {
	return {
		/**
		 * Sends the login mail. New accounts only with a signup code (membership / promo): without an account and without
		 * such a code the answer is still `sent`, but the mail only explains how to get an account (no one-time code).
		 */
		authRequest: (
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
			}),

		/**
		 * `marketing_opt_in`: stored for a new account; `true` also opts an existing account in.
		 * New account with a product code = account + claim in one step: 409 already_claimed / 422 choose_item → no account.
		 * 403 signup_code_required.
		 */
		authVerifyOtp: (email: string, otp: string, marketingOptIn?: boolean) =>
			api.post<VerifyResult>('/auth/verify/', { email, otp, marketing_opt_in: marketingOptIn }),

		authVerifyToken: (token: string) => api.post<VerifyResult>('/auth/verify/', { token }),

		/** Device access check for a code (no login needed; sends the session if there is one). 404 / 410 expired / 429 */
		checkAccess: (code: string) => api.post<AccessResult>('/access/', { code }),

		authLogout: () => api.post('/auth/logout/'),

		fetchMe: () => api.get<Me>('/me/'),

		/**
		 * Account settings: `PUT /me/ { marketing_opt_in?, hide_sensitive? }` → the updated user and avatar (turning
		 * `hide_sensitive` on moves a worn sensitive body to the character's first non-sensitive look)
		 */
		updateMe: (patch: { marketing_opt_in?: boolean; hide_sensitive?: boolean }) =>
			api.put<{ status: string; user: User; avatar?: Avatar }>('/me/', patch),

		fetchCatalog: () => api.get<Catalog>('/items/', { auth: false }),

		fetchEditions: () => api.get<Edition[]>('/editions/', { auth: false }),

		fetchCode: (code: string) =>
			api.get<CodeInfo>(`/code/?c=${encodeURIComponent(code)}`),

		claimCode: (code: string, item_id?: string | null, variant?: string | null) =>
			api.post<InventoryResult>('/inventory/claim/', {
				code,
				item_id: item_id || undefined,
				variant: variant || undefined
			}),

		craftRecipe: (recipe_id: string) =>
			api.post<InventoryResult>('/inventory/craft/', { recipe_id }),

		grantItem: (item_id: string, source: GrantSource, ref?: string) =>
			api.post<InventoryResult>('/inventory/grant/', { item_id, source, ref }),

		importItems: (items: { item_id: string; qty: number }[]) =>
			api.post<InventoryResult>('/inventory/import/', { items }),

		/** apply one piece of a substance (ITEMS.md "Substances"): 422 missing / not_usable */
		useItem: (item_id: string) =>
			api.post<InventoryResult & { item_id: string; stats: Me['stats']; effects: Me['effects'] }>('/inventory/use/', { item_id }),
		/** end a running reader effect ("Effekt beenden") */
		endEffect: (kind: string) => api.post<{ status: string; ended: boolean; effects: Me['effects'] }>('/effects/end/', { kind }),

		/** `restored` = slot instances the cover rule refilled ("Hide sensitive content", ITEMS.md "Wardrobe") */
		putAvatar: (slots: Avatar['slots']) =>
			api.put<{ status: string; avatar: Avatar; restored?: string[] }>('/avatar/', { slots }),
		/** the avatar body (must be unlocked and have an A-pose image: 422 locked / no_apose); null = the default */
		putAvatarBody: (body: AvatarBodyRef | null) =>
			api.put<{ status: string; avatar: Avatar; restored?: string[] }>('/avatar/', { body }),

		fetchProgress: (toonId: string) =>
			api.get<ProgressResult>(`/progress/${encodeURIComponent(toonId)}/`),

		putProgress: (toonId: string, state: any, updated_at: number) =>
			api.put<ProgressResult>(`/progress/${encodeURIComponent(toonId)}/`, { state, updated_at }),
	};
}
