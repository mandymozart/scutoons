import type { MembershipType } from '$lib/models/membership.ts';

/**
 * @param {string} type
 * @returns {type is MembershipType}
 */
export const isValidMembershipType = (type: string): type is MembershipType => {
	return type === 'membership' || type === 'promo';
};

/**
 * @param {string | null} expires_at
 * @returns {boolean}
 */
export const isMembershipExpired = (expires_at: string | null): boolean => {
	if (!expires_at) return false;
	const exp = new Date(expires_at.replace(' ', 'T') + 'Z');
	return Date.now() > exp.getTime();
};
