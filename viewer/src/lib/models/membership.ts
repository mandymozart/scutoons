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
