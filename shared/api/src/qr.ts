// Typed endpoints of the SCUTOONS API: accounts, access, items, inventory, avatar, progress
// (agents/ITEMS.md → HTTP API contract). `createQrApi(client)` binds them to one configured client.
import type { ApiClient } from './client.ts';
import type { CampaignInvite } from './admin.ts';
import type { Avatar, AvatarBodyRef, Catalog, CodeInfo, Edition, Entry, GrantSource, Me, OwnershipClaim, User, RecurrenceInterval } from '@scutoons/content';

export interface VerifyResult {
	status: string;
	session_token: string;
	user: User;
	/** true = the account was just created (signup with a membership or promo code) */
	created?: boolean;
	/** only for claimable codes (a promo code is never owned) */
	claimed?: string[];
	claim_error?: { code: string; error: string; message: string; options?: unknown };
	/** the request was a "This is mine" sign-in (`dispute`): nothing was claimed, file the ownership claim next */
	dispute?: boolean;
}

/** `/auth/session/`: who is signed in, and how (single sign-on cookie or the Bearer token); `user: null` = nobody */
export interface SessionResult {
	status: string;
	user: User | null;
	via: 'cookie' | 'bearer' | null;
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

/** `POST /inventory/gift/` (ITEMS.md "Ownership, claims, gifting") */
export interface GiftResult extends InventoryResult {
	gift: { item_id: string; variant: string | null; qty: number; code: string | null; to: string; transfer_id: number };
}

/** One message of the contact form (POST /api/contact/, server Domain\Contact\ContactForm) */
export interface ContactMessage {
	topic: 'general' | 'code' | 'account' | 'privacy' | 'press';
	email: string;
	name?: string;
	code?: string;
	message: string;
	locale?: string;
	page?: string;
	website: string;
	elapsed: number;
}

export type QrApi = ReturnType<typeof createQrApi>;

/** The campaign a member may invite with (research/GIFT-INVITES.md; server CampaignStore::offerFor) */
export interface InviteOffer {
	id: string;
	ends_at: string | null;
	/** an invite sent now waits until then */
	wait_until: string | null;
	/** own = the member's own item (the gift form), campaign = item_id / variant given by SCUTOONS */
	gift: 'own' | 'campaign';
	item_id: string | null;
	variant: string | null;
	physical: boolean;
	payer: 'scutoons' | 'member' | null;
	/** the member pays this (by hand), null = free */
	price_cents: number | null;
	currency: string;
	/** invites the member has left now */
	left: number;
}

export interface InvitesResult {
	status: string;
	campaign: InviteOffer | null;
	invites: CampaignInvite[];
	/** physical gifts the member joined by that wait for a shipping address */
	address_needed: { id: string; item_id: string; variant: string | null; item: string; countries: string[] }[];
}

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
			choice?: { item_id?: string | null; variant?: string | null; dispute?: boolean }
		) =>
			api.post<{ status: string }>('/auth/request/', {
				email,
				code: code || undefined,
				locale,
				marketing_opt_in: marketingOptIn,
				// the item picked for a code with options (a new account claims it in the same transaction)
				item_id: choice?.item_id || undefined,
				variant: choice?.variant || undefined,
				// "This is mine": an account only to file an ownership claim on a code registered to someone else
				dispute: choice?.dispute || undefined
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

		/** Ends the session (the shared one: every app is signed out) and clears the cookie; `all` = every device */
		authLogout: (all = false) => api.post('/auth/logout/', all ? { all: true } : {}),

		/** Single sign-on: the account of the session cookie alone (never the stored token) – `user: null` when there is none */
		authSession: () => api.get<SessionResult>('/auth/session/', { auth: false }),

		/** Single sign-on: the stored Bearer token of an app → the same session as the cookie (once, then drop it) */
		authExchange: () => api.post<SessionResult>('/auth/session/'),

		fetchMe: () => api.get<Me>('/me/'),

		/**
		 * Account settings: `PUT /me/ { marketing_opt_in?, hide_sensitive?, show_hints?, media_quality?, display_name? }` → the updated user and avatar
		 */
		updateMe: (patch: { marketing_opt_in?: boolean; hide_sensitive?: boolean; show_hints?: boolean; media_quality?: User['media_quality']; display_name?: string | null }) =>
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

		/** `use` = { <item id>: <entry id> }: which entry of an input (the plastic's colour, the book) – none = the oldest */
		craftRecipe: (recipe_id: string, use?: Record<string, string | number>) =>
			api.post<InventoryResult>('/inventory/craft/', { recipe_id, ...(use && Object.keys(use).length ? { use } : {}) }),

		/**
		 * Into a container or out (`into` null) – research/SCORES.md §2: 409 does_not_fit / container_full /
		 * container_in_container / worn, 409 overfill_warning { max, days } until `overfill: true`
		 */
		moveEntry: (entry_id: string | number, into: string | number | null, overfill = false) =>
			api.post<InventoryResult>('/inventory/move/', { entry_id, into, ...(overfill ? { overfill } : {}) }),

		/** a score's MusicXML / PDF as a short-lived signed URL (the PDF only for a signed-in owner) */
		scoreFile: (item_id: string, file: 'musicxml' | 'pdf') =>
			api.get<{ status: string; url: string; expires: number }>(`/items/${encodeURIComponent(item_id)}/score/${file}`),

		/** `respawn` (+ `visit` for every visit): a placed item that reappears – the server checks it and adds the period to the ref */
		grantItem: (item_id: string, source: GrantSource, ref?: string, respawn?: { respawn: RecurrenceInterval; visit?: string }, variant?: string | null) =>
			api.post<InventoryResult & { granted?: number }>('/inventory/grant/', { item_id, source, ref, ...(respawn ?? {}), ...(variant ? { variant } : {}) }),

		/** a story takes an item (BACKLOG 10.38): all or nothing; `ref` = once per ref (the repeat rule's period) */
		takeItem: (item_id: string, qty: number, ref?: string) =>
			api.post<InventoryResult & { taken: number; reason: 'too_few' | 'already_taken' | null }>('/inventory/take/', { item_id, qty, ref }),

		/** `local_id` / `in`: a guest's container and what lay in it (research/SCORES.md §2) */
		importItems: (items: { item_id: string; qty: number; variant?: string | null; local_id?: string; in?: string }[]) =>
			api.post<InventoryResult>('/inventory/import/', { items }),

		/** apply one piece of a substance (ITEMS.md "Substances"): 422 missing / not_usable */
		useItem: (item_id: string) =>
			api.post<InventoryResult & { item_id: string; stats: Me['stats']; effects: Me['effects'] }>('/inventory/use/', { item_id }),
		/** end a running reader effect ("Effekt beenden") */
		endEffect: (kind: string) => api.post<{ status: string; ended: boolean; effects: Me['effects'] }>('/effects/end/', { kind }),

		/**
		 * Gives an item to another reader (ITEMS.md "Ownership, claims, gifting"): an entry (a code-bound piece moves its
		 * code) or `item_id` (+ variant), `qty` for stackables, `to` = their email. 404 recipient_not_found (no account) ·
		 * 403 not_giftable · 409 worn / cooldown / max_reached · 400 self_gift / invalid_email · 429 daily_limit
		 */
		giftItem: (gift: { entry_id?: string | number; item_id?: string; variant?: string | null; qty?: number; to: string }) =>
			api.post<GiftResult>('/inventory/gift/', gift),
		/** gift invites (research/GIFT-INVITES.md): the running campaign, the member's invites, gifts waiting for an address */
		invites: () => api.get<InvitesResult>('/invites/'),
		/** an invite: the member's own item (entry_id / item_id) or, in a campaign item campaign, only `to` */
		createInvite: (invite: { to: string; entry_id?: string | number; item_id?: string; variant?: string | null; qty?: number }) =>
			api.post<InventoryResult & { invite: CampaignInvite }>('/invites/', invite),
		/** calls an invite back while it is not accepted (the own item comes back) */
		recallInvite: (id: string) => api.post<InventoryResult & { invite: CampaignInvite }>(`/invites/${encodeURIComponent(id)}/recall/`),
		/** the shipping address of a physical gift the member joined by */
		inviteAddress: (id: string, address: { name: string; address: string; country: string }) =>
			api.put<InvitesResult>(`/invites/${encodeURIComponent(id)}/address/`, address),
		/**
		 * The contact form (POST /api/contact/): one mail to hello@ with the sender as Reply-To. `website` = the honeypot
		 * (empty), `elapsed` = ms since the form opened. 400 invalid_email / message_required / message_too_long ·
		 * 429 too many · 503 mail_failed
		 */
		sendContact: (message: ContactMessage) => api.post<{ status: string }>('/contact/', message),
		/** marks the `received` notices of `/me/` as seen (all when no ids) */
		markReceivedSeen: (ids?: number[]) => api.post<{ status: string; seen: number }>('/inventory/received/seen/', ids ? { ids } : {}),

		/** "This is mine": a claim on a code registered to another account. 409 not_registered / already_yours / claim_open */
		fileClaim: (code: string, statement: string) =>
			api.post<{ status: string; claim: OwnershipClaim }>('/claims/', { code, statement }),
		/** evidence for an open claim (JPEG, PNG, WebP or PDF; 413 file_too_large, 415 unsupported_type, 409 too_many_files) */
		addClaimFile: (claimId: string, file: File | Blob, name?: string) =>
			api.post<{ status: string; claim: OwnershipClaim }>(
				`/claims/${encodeURIComponent(claimId)}/files?name=${encodeURIComponent(name ?? (file as File).name ?? 'file')}`,
				file
			),
		myClaims: () => api.get<{ status: string; items: OwnershipClaim[] }>('/claims/'),
		withdrawClaim: (claimId: string) =>
			api.post<{ status: string; claim: OwnershipClaim }>(`/claims/${encodeURIComponent(claimId)}/withdraw`),

		putAvatar: (slots: Avatar['slots']) =>
			api.put<{ status: string; avatar: Avatar }>('/avatar/', { slots }),
		/** the avatar body (must be unlocked and have an A-pose image: 422 locked / no_apose); null = the default */
		putAvatarBody: (body: AvatarBodyRef | null) =>
			api.put<{ status: string; avatar: Avatar }>('/avatar/', { body }),

		fetchProgress: (toonId: string) =>
			api.get<ProgressResult>(`/progress/${encodeURIComponent(toonId)}/`),

		putProgress: (toonId: string, state: any, updated_at: number) =>
			api.put<ProgressResult>(`/progress/${encodeURIComponent(toonId)}/`, { state, updated_at }),
	};
}
