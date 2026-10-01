// Who may use which app (single sign-on: one session, every app checks its own roles). The server checks the same roles
// again on every request (server/docs/API.md "Roles"); this matrix only decides what an app shows.

export type AppId = 'viewer' | 'shop' | 'creator' | 'admin' | 'embassy';

/** Roles that open an app (any of them); `null` = every signed-in account (and guests, where the app allows it) */
export const APP_ROLES: Record<AppId, readonly string[] | null> = {
	viewer: null,
	shop: null,
	// the content API (/api/entities) requires `creator` itself – `admin` alone would only see 403s
	creator: ['creator'],
	admin: ['admin'],
	embassy: ['liaison', 'ambassador', 'admin']
};

/** Does an account with these roles get into the app? */
export function canUseApp(app: AppId, roles: readonly string[]): boolean {
	const needed = APP_ROLES[app];
	return needed === null || needed.some((role) => roles.includes(role));
}
