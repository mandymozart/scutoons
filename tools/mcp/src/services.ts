// Who the member is and which SCUTOONS services their roles open (agents/MCP.md): the server lists only the tools of
// those services – the API's role checks still decide every call. Readers (no member role) can't connect (owner
// 2026-10-09). Plus the app links every draft is handed back with (the member confirms in the browser).
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { parseEntityKey } from '@scutoons/content';
import { ROOT, type Config } from './config.ts';

export interface Member {
	id: string;
	email: string;
	/** users.display_name ('' = none set) – the Embassy signs with its first name */
	name: string;
	roles: string[];
}

export interface Services {
	/** creator → write; admin without creator → read (the API lets admins edit, the admin app doesn't – Ask did the same) */
	content: 'write' | 'read' | null;
	embassy: boolean;
	admin: boolean;
	/** the database migrations (role developer, BACKLOG 10.61) */
	developer: boolean;
}

/** the roles that open a service (Domain\Embassy: liaison / ambassador / admin; RequireRole admin; requireContentEditor) */
export function servicesOf(roles: readonly string[]): Services {
	const has = (r: string) => roles.includes(r);
	return {
		content: has('creator') ? 'write' : has('admin') ? 'read' : null,
		embassy: has('liaison') || has('ambassador') || has('admin'),
		admin: has('admin'),
		developer: has('developer')
	};
}
export const hasAny = (s: Services) => !!s.content || s.embassy || s.admin;

/** the member behind the token (API /me/); null = not signed in */
export async function fetchMember(config: Config): Promise<Member | null> {
	const res = await fetch(`${config.api}/me/`, { headers: { Authorization: `Bearer ${config.token}`, Accept: 'application/json' } });
	if (res.status === 401) return null;
	if (!res.ok) throw new Error(`${config.api}/me/ answered ${res.status}`);
	const user = ((await res.json()) as any)?.user;
	if (!user) return null;
	return {
		id: String(user.id),
		email: String(user.email ?? ''),
		name: typeof user.display_name === 'string' ? user.display_name.trim() : '',
		roles: Array.isArray(user.roles) ? user.roles.map(String) : []
	};
}

export interface AppLinks {
	creator: string;
	admin: string;
	embassy: string;
	viewer: string;
}

/** the apps' origins: the local dev ports against a local API, else production (server config/settings.php *_URL) */
export function appLinks(config: Config): AppLinks {
	const production = { creator: 'https://creator.scutoons.com', admin: 'https://admin.scutoons.com', embassy: 'https://embassy.scutoons.com', viewer: 'https://scutoons.com' };
	if (!config.local) return production;
	let ports: Record<string, number>;
	try {
		ports = JSON.parse(readFileSync(resolve(ROOT, 'shared/dev-ports.json'), 'utf8'));
	} catch {
		return production; // the one-file build outside a checkout
	}
	const at = (app: string) => `http://localhost:${ports[app]}`;
	return { creator: at('creator'), admin: at('admin'), embassy: at('embassy'), viewer: at('viewer') };
}

/** the creator page of a stored document (where the member checks a draft and publishes); null = no page of its own */
export function creatorHref(links: AppLinks, key: string): string | null {
	const ref = parseEntityKey(key);
	if (!ref?.toon) return null;
	const base = `${links.creator}/${ref.toon}`;
	switch (ref.kind) {
		case 'chapter':
			return `${base}/chapter/${ref.number}`;
		case 'storyboard':
			return `${base}/chapter/${ref.number}/storyboard`;
		case 'season':
			return `${base}/season/${ref.number}`;
		case 'script':
		case 'cast':
		case 'credits':
			return `${base}/${ref.kind}`;
		case 'toon':
		case 'toon-settings':
			return `${base}/toon`;
		default:
			return null;
	}
}
