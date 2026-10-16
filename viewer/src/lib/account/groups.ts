// The account page's groups (owner 2026-10-16, concept B of agents/research/ACCOUNT-PAGE.md): an index of groups, each
// its own sub-page `/account/<id>` (new URLs; `/account` stays the index). Order = the index's order.

/** who reads – decides which groups show */
export interface AccountReader {
	signedIn: boolean;
	roles: readonly string[];
	/** may use the creator (`canUseApp('creator', roles)`) */
	creator: boolean;
	/** the browser can install the app (or show the iOS how-to) or save chapters (else "Offline content" is empty) */
	offline: boolean;
	/** this device can get push notifications (lib/push.svelte.ts state ≠ hidden) – guests too, it is per device */
	push: boolean;
}

export const ACCOUNT_GROUPS = ['details', 'reading', 'sound', 'offline', 'notifications', 'collection', 'connections', 'creator'] as const;
export type AccountGroup = (typeof ACCOUNT_GROUPS)[number];

/** a Feather icon per group (shown before its title in the index and on its page) */
export const GROUP_ICON: Record<AccountGroup, string> = {
	details: 'user',
	reading: 'book-open',
	sound: 'volume-2',
	offline: 'download-cloud',
	notifications: 'bell',
	collection: 'package',
	connections: 'link',
	creator: 'edit-3'
};

/** where a group's settings are kept: one badge per group, not a sentence per setting; mixed groups have none */
export function groupScope(group: AccountGroup, reader: AccountReader): 'account' | 'device' | null {
	if (group === 'sound' || group === 'offline') return 'device';
	if (group === 'reading') return reader.signedIn ? 'account' : 'device';
	return null;
}

/** the roles that may connect their own AI (server ApiTokens::MEMBER_ROLES) */
const AI_ROLES = ['creator', 'liaison', 'ambassador', 'admin'];

export function groupVisible(group: AccountGroup, reader: AccountReader): boolean {
	switch (group) {
		case 'offline':
			return reader.offline;
		case 'notifications':
			return reader.signedIn || reader.push;
		case 'connections':
			return reader.signedIn && reader.roles.some((r) => AI_ROLES.includes(r));
		case 'creator':
			return reader.signedIn && reader.creator;
		default:
			return true;
	}
}

export function isAccountGroup(value: string | undefined): value is AccountGroup {
	return (ACCOUNT_GROUPS as readonly string[]).includes(value ?? '');
}
