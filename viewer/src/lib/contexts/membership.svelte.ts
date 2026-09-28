import { setContext, getContext } from 'svelte';
import { ApiError } from '$lib/api/client.ts';
import { claimCode } from '$lib/api/qr.service.ts';
import type { SessionContext } from './session.svelte.ts';

const MEMBERSHIP_KEY = Symbol('membership');
/** legacy device-local membership (pre-accounts); now only a pending claim */
const LEGACY_KEY = 'scutoons_membership';
const PROMPTED_KEY = 'scutoons_membership_prompted';

export type MembershipContext = ReturnType<typeof createMembershipContext>;

/**
 * Membership = a valid membership of the logged-in account (`/me/` memberships). A code belongs to exactly
 * one account, so nothing is granted to guests. A legacy `scutoons_membership` code is kept as a pending
 * claim: claimed silently on sign-in, guests get a one-time prompt to sign in.
 */
export const createMembershipContext = (session: SessionContext) => {
	let pendingCode = $state<string | null>(null);
	let prompted = $state(true);
	/** legacy code that turned out to belong to another account (message shown once) */
	let conflictCode = $state<string | null>(null);
	let migrating = false;

	const isValid = $derived(session.hasValidMembership);
	const showLegacyPrompt = $derived(!!pendingCode && !prompted && session.ready && !session.isLoggedIn);

	function read(key: string) {
		try {
			return localStorage.getItem(key);
		} catch {
			return null;
		}
	}

	function clearLegacy() {
		pendingCode = null;
		try {
			localStorage.removeItem(LEGACY_KEY);
			localStorage.removeItem(PROMPTED_KEY);
		} catch {
			/* storage unavailable */
		}
	}

	async function migrate() {
		if (!pendingCode || migrating || !session.isLoggedIn) return;
		migrating = true;
		const code = pendingCode;
		try {
			await claimCode(code); // idempotent for the owner
			clearLegacy();
			await session.loadMe();
		} catch (e) {
			if (e instanceof ApiError && (e.status === 409 || e.code === 'already_claimed')) {
				conflictCode = code;
				clearLegacy();
			} else if (e instanceof ApiError && e.status === 422) {
				// needs a choice (e.g. item/variant): hand over to the claim sheet via the prompt
				prompted = false;
			} else {
				console.warn('Legacy membership claim failed, will retry', e);
			}
		} finally {
			migrating = false;
		}
	}

	const initFromStorage = () => {
		const stored = read(LEGACY_KEY);
		if (stored) {
			try {
				pendingCode = JSON.parse(stored)?.code || null;
			} catch {
				pendingCode = null;
			}
			if (!pendingCode) clearLegacy();
		}
		prompted = read(PROMPTED_KEY) === '1';
		session.subscribe((event) => {
			if (event === 'login' || event === 'me') migrate();
		});
		migrate();
	};

	return {
		get isValid() {
			return isValid;
		},
		get pendingCode() {
			return pendingCode;
		},
		get showLegacyPrompt() {
			return showLegacyPrompt;
		},
		get conflictCode() {
			return conflictCode;
		},
		dismissPrompt() {
			prompted = true;
			try {
				localStorage.setItem(PROMPTED_KEY, '1');
			} catch {
				/* storage unavailable */
			}
		},
		clearLegacy,
		dismissConflict() {
			conflictCode = null;
		},
		initFromStorage
	};
};

export const setMembershipContext = (session: SessionContext) => {
	const context = createMembershipContext(session);
	setContext(MEMBERSHIP_KEY, context);
	return context;
};

export const getMembershipContext = () => {
	return getContext<MembershipContext>(MEMBERSHIP_KEY);
};
