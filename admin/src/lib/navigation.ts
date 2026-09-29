// Navigation as data (admin UI pattern from akin): groups → items { title, description, href, icon }. The shell and
// the command palette both render from this list; titles/descriptions are paraglide messages (functions).
import { m } from '$lib/paraglide/messages.js';

export interface NavItem {
	id: string;
	title: () => string;
	description: () => string;
	href: string;
	/** a single glyph (the shell has no icon set of its own) */
	icon: string;
	/** extra words for the command palette */
	keywords?: string;
}

export interface NavGroup {
	id: string;
	title: () => string;
	items: NavItem[];
}

export const navigation: NavGroup[] = [
	{
		id: 'overview',
		title: () => m['nav.group_overview'](),
		items: [
			{ id: 'dashboard', title: () => m['nav.dashboard'](), description: () => m['nav.dashboard_hint'](), href: '/', icon: '▦', keywords: 'kpi stats' }
		]
	},
	{
		id: 'codes',
		title: () => m['nav.group_codes'](),
		items: [
			{ id: 'codes', title: () => m['nav.codes'](), description: () => m['nav.codes_hint'](), href: '/codes', icon: '▣', keywords: 'qr revoke csv' },
			{ id: 'generate', title: () => m['nav.generate'](), description: () => m['nav.generate_hint'](), href: '/codes/new', icon: '+', keywords: 'new code certificate' },
			{ id: 'batches', title: () => m['nav.batches'](), description: () => m['nav.batches_hint'](), href: '/batches', icon: '▤', keywords: 'zip print sheet edition' }
		]
	},
	{
		id: 'people',
		title: () => m['nav.group_people'](),
		items: [
			{ id: 'users', title: () => m['nav.users'](), description: () => m['nav.users_hint'](), href: '/users', icon: '◉', keywords: 'accounts roles email' },
			{ id: 'events', title: () => m['nav.events'](), description: () => m['nav.events_hint'](), href: '/events', icon: '≡', keywords: 'log activity scans' }
		]
	},
	{
		id: 'data',
		title: () => m['nav.group_data'](),
		items: [
			{ id: 'content', title: () => m['nav.content'](), description: () => m['nav.content_hint'](), href: '/content', icon: '¶', keywords: 'chapters toons versions publish' },
			{ id: 'catalog', title: () => m['nav.catalog'](), description: () => m['nav.catalog_hint'](), href: '/catalog', icon: '◇', keywords: 'items editions permissions' }
		]
	}
];

export const navItems = navigation.flatMap((group) => group.items);

/** The active item: the longest href that prefixes the path ("/" only for the dashboard itself) */
export function activeItem(pathname: string): NavItem | undefined {
	return navItems
		.filter((item) => (item.href === '/' ? pathname === '/' : pathname === item.href || pathname.startsWith(item.href + '/')))
		.sort((a, b) => b.href.length - a.href.length)[0];
}
