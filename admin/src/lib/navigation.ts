// Navigation as data (admin UI pattern from akin): groups → items { title, description, href, icon }. The shell and
// the command palette both render from this list; titles/descriptions are paraglide messages (functions).
import { m } from '$lib/paraglide/messages.js';
import { openClaims } from '$lib/claims.svelte.ts';

export interface NavItem {
	id: string;
	title: () => string;
	description: () => string;
	href: string;
	/** a Feather icon name (`Icon` from @scutoons/ui, DESIGN.md "Icons") */
	icon: string;
	/** extra words for the command palette */
	keywords?: string;
	/** a count shown as a badge beside the title (e.g. open claims); null / 0 = none */
	count?: () => number | null;
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
			{ id: 'dashboard', title: () => m['nav.dashboard'](), description: () => m['nav.dashboard_hint'](), href: '/', icon: 'bar-chart-2', keywords: 'kpi stats' },
			{ id: 'proposals', title: () => m['nav.proposals'](), description: () => m['nav.proposals_hint'](), href: '/proposals', icon: 'inbox', keywords: 'ai claude mcp apply vorschlag' }
		]
	},
	{
		id: 'codes',
		title: () => m['nav.group_codes'](),
		items: [
			{ id: 'codes', title: () => m['nav.codes'](), description: () => m['nav.codes_hint'](), href: '/codes', icon: 'hash', keywords: 'qr revoke csv' },
			{ id: 'generate', title: () => m['nav.generate'](), description: () => m['nav.generate_hint'](), href: '/codes/new', icon: 'plus-square', keywords: 'new code certificate' },
			{ id: 'batches', title: () => m['nav.batches'](), description: () => m['nav.batches_hint'](), href: '/batches', icon: 'layers', keywords: 'zip print sheet edition promo codes promotion' },
			{
				id: 'claims',
				title: () => m['nav.claims'](),
				description: () => m['nav.claims_hint'](),
				href: '/claims',
				icon: 'flag',
				keywords: 'claim ownership dispute transfer stolen anspruch besitz',
				count: () => openClaims.count
			}
		]
	},
	{
		id: 'people',
		title: () => m['nav.group_people'](),
		items: [
			{ id: 'users', title: () => m['nav.users'](), description: () => m['nav.users_hint'](), href: '/users', icon: 'users', keywords: 'accounts roles email' },
			{ id: 'campaigns', title: () => m['nav.campaigns'](), description: () => m['nav.campaigns_hint'](), href: '/campaigns', icon: 'gift', keywords: 'campaign invite invites gift promotion segment kampagne einladung geschenk werbung' },
			{ id: 'reading', title: () => m['nav.reading'](), description: () => m['nav.reading_hint'](), href: '/reading', icon: 'book-open', keywords: 'progress chapters readers lesen kapitel fortschritt lesestand' },
			{ id: 'events', title: () => m['nav.events'](), description: () => m['nav.events_hint'](), href: '/events', icon: 'list', keywords: 'log activity scans' }
		]
	},
	{
		id: 'data',
		title: () => m['nav.group_data'](),
		items: [
			{ id: 'assets', title: () => m['nav.assets'](), description: () => m['nav.assets_hint'](), href: '/assets', icon: 'image', keywords: 'media cdn files images videos bunny upload storage bandwidth' },
			{ id: 'old-versions', title: () => m['nav.old_versions'](), description: () => m['nav.old_versions_hint'](), href: '/old-versions', icon: 'clock', keywords: 'history purge delete drafts versions database cleanup alte fassungen verlauf löschen aufräumen' },
			{ id: 'catalog', title: () => m['nav.catalog'](), description: () => m['nav.catalog_hint'](), href: '/catalog', icon: 'package', keywords: 'items recipes albums products editions toons permissions code types meta katalog rezepte alben produkte editionen berechtigungen' },
			{ id: 'calendar', title: () => m['nav.calendar'](), description: () => m['nav.calendar_hint'](), href: '/calendar', icon: 'calendar', keywords: 'events termine gigs concerts shows dates konzerte veranstaltungen' },
			{ id: 'homepage', title: () => m['nav.homepage'](), description: () => m['nav.homepage_hint'](), href: '/homepage', icon: 'home', keywords: 'startseite landing featured products editions slider shop' }
		]
	},
	{
		id: 'system',
		title: () => m['nav.group_system'](),
		items: [
			{ id: 'settings', title: () => m['nav.settings'](), description: () => m['nav.settings_hint'](), href: '/settings', icon: 'settings', keywords: 'rate limit scans lookups event wifi config' },
			{ id: 'emails', title: () => m['nav.emails'](), description: () => m['nav.emails_hint'](), href: '/emails', icon: 'mail', keywords: 'mail email log sent failed templates preview recipients smtp login code newsletter contacts unsubscribe bounce import kontakte abmelden' },
			{ id: 'server-errors', title: () => m['nav.server_errors'](), description: () => m['nav.server_errors_hint'](), href: '/server-errors', icon: 'alert-triangle', keywords: 'errors log php fatal warning exception 500 crash qr api serverfehler protokoll' }
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
