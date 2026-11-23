import { entities } from '@scutoons/content';
import { pwaLabels } from '@scutoons/pwa';
import { CATALOG_LABELS, isCatalogPageKind } from '$lib/editor/catalog-docs.ts';
import { fileLabel } from '$lib/labels.ts';
import type { CreatorNav, Crumb } from '$lib/nav.svelte.ts';
import { m } from '$lib/paraglide/messages.js';
import { getLocale } from '$lib/paraglide/runtime.js';

/**
 * The path of a route – the area, then toon / season / chapter (a page adds deeper steps itself: the chapter editor's
 * group and slice). The front tab carries it (its steps go back up), Recent keeps it, the window title ends with it.
 */
export function routeCrumbs(route: string, params: Record<string, string | undefined>, url: URL, nav: CreatorNav): Crumb[] {
	const out: Crumb[] = [];
	const toonId = params.toonId ?? null;
	if (route === '/toons' || toonId) out.push({ href: '/toons', label: m['creator.toons']() });
	if (route === '/notifications') out.push({ href: '/notifications', label: pwaLabels(getLocale()).notifications });
	if (route === '/guide') out.push({ href: '/guide', label: m['creator.style_guide']() });
	if (route === '/characters') {
		out.push({ href: '/characters', label: m['creator.characters']() });
	}
	if (route === '/map') out.push({ href: '/map', label: m['creator.map']() });
	if (route === '/generate') out.push({ href: '/generate', label: m['shell.ws_generate']() });
	if (route === '/scenes/[[id]]') out.push({ href: '/scenes', label: m['creator.scenes']() });
	if (route === '/experiences/[[id]]') out.push({ href: '/experiences', label: m['creator.experiences']() });
	if (route === '/catalog/[kind]' || route === '/catalog/items/[id]/collage') {
		out.push({ href: '/catalog/items', label: m['creator.catalog']() });
		const kind = params.kind ?? 'items';
		if (isCatalogPageKind(kind)) out.push({ href: `/catalog/${kind}`, label: CATALOG_LABELS[kind]() });
	}
	if (route === '/kanon/[subject]') {
		const subject = params.subject ?? '';
		const area = subject.split('.')[0];
		if (area === 'character') out.push({ href: '/characters', label: m['creator.characters']() });
		if (area === 'place') out.push({ href: '/map', label: m['creator.map']() });
		if (area === 'toon') out.push({ href: '/toons', label: m['creator.toons']() }, { href: `/${subject.slice(5)}`, label: nav.toonName(subject.slice(5)) });
		out.push({ href: `/kanon/${subject}`, label: fileLabel(entities.kanon(subject)) });
	}
	if (route === '/flow') out.push({ href: '/flow', label: m['creator.flow_universe']() });
	if (!toonId) return out;
	out.push({ href: `/${toonId}`, label: nav.toonName(toonId) });
	// a chapter's season: the season whose sequence lists it (none lists it / still loading: no season step)
	const chapterSeason = params.chapterNumber ? nav.chapterSeason(toonId, Number(params.chapterNumber)) : null;
	const season = params.seasonNumber ?? (chapterSeason != null ? String(chapterSeason) : null);
	if (season) out.push({ href: `/${toonId}/season/${season}`, label: fileLabel(entities.season(toonId, Number(season))) });
	if (route === '/[toonId]/season/[seasonNumber]/cover') out.push({ href: `/${toonId}/season/${season}/cover`, label: m['creator.cover_editor']() });
	if (route === '/[toonId]/season/[seasonNumber]/script') out.push({ href: `/${toonId}/season/${season}/script`, label: m['creator.script']() });
	if (params.chapterNumber) {
		const n = params.chapterNumber;
		out.push({ href: `/${toonId}/chapter/${n}`, label: fileLabel(entities.chapter(toonId, Number(n))) });
		if (route === '/[toonId]/chapter/[chapterNumber]/script') out.push({ href: `/${toonId}/chapter/${n}/script`, label: m['creator.script']() });
		if (route === '/[toonId]/chapter/[chapterNumber]/storyboard') out.push({ href: `/${toonId}/chapter/${n}/storyboard`, label: m['storyboard.title']() });
	}
	if (route === '/[toonId]/credits') out.push({ href: `/${toonId}/credits`, label: m['creator.credits']() });
	if (route === '/[toonId]/script') out.push({ href: `/${toonId}/script`, label: m['creator.script']() });
	if (route === '/[toonId]/toon') out.push({ href: `/${toonId}/toon`, label: m['creator.toon_settings']() });
	return out;
}

/** the path the front tab shows: the areas' roots (Toons) are Home's, not steps */
export const tabPath = (crumbs: Crumb[]): Crumb[] => crumbs.filter((c) => c.href !== '/' && c.href !== '/toons');

/** what the tab of `itemHref` is called: its own step of the path, else the deepest step */
export function tabLabel(crumbs: Crumb[], itemHref: string | null): string {
	const path = itemHref?.split('?')[0];
	return (itemHref && (crumbs.find((c) => c.href === itemHref) ?? (!itemHref.includes('?') ? crumbs.find((c) => c.href.split('?')[0] === path) : undefined))?.label) || crumbs.at(-1)?.label || '';
}
