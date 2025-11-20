import { setContext, getContext } from 'svelte';
import { validateQRCode } from '$lib/api/membership.service.ts';
import { isValidMembershipType, isMembershipExpired } from '$lib/auth/index.ts';
import type { Membership } from '$lib/models/membership.ts';

const MEMBERSHIP_KEY = Symbol('membership');
const STORAGE_KEY = 'scutoons_membership';

/**
 * @returns {ReturnType<typeof createMembershipContext>}
 */
export const createMembershipContext = () => {
	let membership = $state<Membership | null>(null);

	let isValid = $derived(
		membership !== null &&
			!isMembershipExpired(membership.expires_at) &&
			isValidMembershipType(membership.type)
	);

	/**
	 * @returns {void}
	 */
	const initFromStorage = () => {
		if (typeof localStorage === 'undefined') return;

		const stored = localStorage.getItem(STORAGE_KEY);
		if (!stored) return;

		try {
			const parsed = JSON.parse(stored);
			if (isValidMembershipType(parsed.type) && !isMembershipExpired(parsed.expires_at)) {
				membership = parsed;
			} else {
				clear();
			}
		} catch {
			clear();
		}
	};

	/**
	 * @param {Membership} data
	 * @returns {void}
	 */
	const persist = (data: Membership) => {
		if (typeof localStorage === 'undefined') return;
		localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
	};

	/**
	 * @returns {void}
	 */
	const clear = () => {
		membership = null;
		if (typeof localStorage !== 'undefined') {
			localStorage.removeItem(STORAGE_KEY);
		}
	};

	/**
	 * @param {string} code
	 * @returns {Promise<import('$lib/models/membership').ValidationResult>}
	 */
	const validate = async (code: string) => {
		const result = await validateQRCode(code);

		if (result.success) {
			membership = result.membership;
			persist(result.membership);
		}

		return result;
	};

	return {
		get membership() {
			return membership;
		},
		get isValid() {
			return isValid;
		},
		validate,
		clear,
		initFromStorage
	};
};

/**
 * @returns {ReturnType<typeof createMembershipContext>}
 */
export const setMembershipContext = () => {
	const context = createMembershipContext();
	setContext(MEMBERSHIP_KEY, context);
	return context;
};

/**
 * @returns {ReturnType<typeof createMembershipContext>}
 */
export const getMembershipContext = () => {
	return getContext<ReturnType<typeof createMembershipContext>>(MEMBERSHIP_KEY);
};
