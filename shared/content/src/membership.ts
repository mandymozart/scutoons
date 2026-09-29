export type MembershipType = 'membership' | 'promo';

export interface Membership {
	type: MembershipType;
	code: string;
	expires_at: string;
	validated_at: string;
	email?: string;
	claimed_at?: string;
	redirect_to: string;
	created_at: string;
}

export type ValidationResult =
	| { success: true; membership: Membership }
	| { success: false; message: string; data?: any; error?: any };

export const isValidMembershipType = (type: string): type is MembershipType => {
	return type === 'membership' || type === 'promo';
};

/** `expires_at` as stored by the server (`YYYY-MM-DD HH:MM:SS`, UTC); null = never expires. */
export const isMembershipExpired = (expires_at: string | null): boolean => {
	if (!expires_at) return false;
	const exp = new Date(expires_at.replace(' ', 'T') + 'Z');
	return Date.now() > exp.getTime();
};
