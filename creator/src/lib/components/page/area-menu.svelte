<script lang="ts" module>
	export type AreaId = 'dashboard' | 'toons' | 'characters' | 'map' | 'catalog' | 'flow';
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { Icon, ToonAvatar } from '@scutoons/ui';
	import { toonName, type ToonIndex } from '@scutoons/content';
	import { asset } from '$lib/utils/cdn.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { recent } from '$lib/recent.svelte.ts';
	import { routeId } from '$lib/nav.svelte.ts';

	/**
	 * The creator's menu by areas (CREATOR-NAV.md §2a, draft D; agents/DESIGN.md "Creator navigation"): Dashboard,
	 * Toons with every toon flat under it (never deeper – a toon's pages are its tabs), Characters, Map, Catalog, Flow;
	 * the place you are in inverted (inside a toon: the toon). The same list everywhere:
	 * - `menu` (desktop): the words, the account and the fold button at its foot;
	 * - `rail` (desktop, folded by hand – Ctrl + \): the areas' icons, the account and the unfold button at its foot;
	 * - `drawer` (phones, from the right like the reader's): the areas, "Recent", then the page's own tools at the foot
	 *   (`foot`: search, notifications, the text language, the page's Flow).
	 * Every entry carries `data-drawer-pick` (a pick closes the phone drawer).
	 */
	let {
		variant,
		toons,
		toon,
		account,
		badge = 0,
		onfold,
		list,
		foot
	}: {
		variant: 'menu' | 'rail' | 'drawer';
		toons: ToonIndex[];
		/** the toon the route is in */
		toon: string | null;
		/** the signed-in account (its e-mail) – opens Notifications */
		account: string;
		/** the dashboard's count (what waits for this member) */
		badge?: number;
		/** fold (menu) or unfold (rail) */
		onfold?: () => void;
		/** the drawer: the page's own list (the chapter tree …) under the areas */
		list?: Snippet;
		foot?: Snippet;
	} = $props();

	const route = $derived(routeId(page.route.id));
	/** the area of the route (a toon's pages belong to Toons) */
	const area = $derived<AreaId | null>(
		route === '/'
			? 'dashboard'
			: route === '/toons' || route.startsWith('/[toonId]')
				? 'toons'
				: route === '/characters'
					? 'characters'
					: route === '/map'
						? 'map'
						: route.startsWith('/catalog/')
							? 'catalog'
							: route === '/flow'
								? 'flow'
								: null
	);
	const areas = $derived<{ id: AreaId; href: string; icon: string; label: string; title?: string }[]>([
		{ id: 'dashboard', href: '/', icon: 'home', label: m['creator.dashboard']() },
		{ id: 'toons', href: '/toons', icon: 'book-open', label: m['creator.toons']() },
		{ id: 'characters', href: '/characters', icon: 'users', label: m['creator.characters']() },
		{ id: 'map', href: '/map', icon: 'map', label: m['creator.map']() },
		{ id: 'catalog', href: '/catalog/items', icon: 'package', label: m['creator.catalog'](), title: m['creator.catalog_hint']() },
		{ id: 'flow', href: '/flow', icon: 'git-merge', label: m['creator.flow_universe'](), title: m['creator.flow_universe_hint']() }
	]);
	/** inverted: the area – inside a toon the toon, not Toons (the rail has no toons: there Toons) */
	const current = (id: AreaId) => (id === area && !(id === 'toons' && toon && variant === 'menu') ? 'page' : undefined);
	const name = (t: ToonIndex) => toonName(t, getLocale());
	const recentPlaces = $derived(recent.list.filter((p) => p.href.split('?')[0] !== page.url.pathname).slice(0, 5));
	const accountLabel = $derived(m['creator.account']({ account }));
</script>

{#if variant === 'rail'}
	<div class="rail" data-testid="shell-rail">
		<ul>
			{#each areas as a (a.id)}
				<li>
					<a class="sq" href={a.href} title={a.label} aria-label={a.label} aria-current={current(a.id)} data-testid="nav-area-{a.id}">
						<Icon name={a.icon} />
						{#if a.id === 'dashboard' && badge}<span class="badge">{badge}</span>{/if}
					</a>
				</li>
			{/each}
		</ul>
		<div class="foot">
			<a class="sq" href="/notifications" title={accountLabel} aria-label={accountLabel}><Icon name="user" /></a>
			<button type="button" class="sq quiet" onclick={() => onfold?.()} aria-label={m['creator.unfold_menu']()} title="{m['creator.unfold_menu']()} (Ctrl + \)" aria-expanded="false" data-testid="menu-unfold"><Icon name="chevrons-right" /></button>
		</div>
	</div>
{:else}
	<div class="menu" class:drawer={variant === 'drawer'}>
		<div class="body">
			<ul class="areas">
				{#each areas as a (a.id)}
					<li>
						<a class="sc-nav-item" href={a.href} title={a.title} aria-current={current(a.id)} data-drawer-pick data-testid="nav-area-{a.id}">
							<Icon name={a.icon} />
							<span class="ell">{a.label}</span>
							{#if a.id === 'dashboard' && badge}<span class="badge">{badge}</span>{/if}
						</a>
					</li>
					{#if a.id === 'toons' && variant === 'menu'}
						{#each toons as t (t.id)}
							<li>
								<a class="sc-nav-item sub" href="/{t.id}" aria-current={toon === t.id || toon === t.uri ? 'page' : undefined} data-testid="nav-toon-{t.id}">
									<ToonAvatar src={t.avatar ? asset(`${t.id}/${t.avatar}`) : null} name={name(t)} size="1.5rem" />
									<span class="ell">{name(t)}</span>
								</a>
							</li>
						{/each}
					{/if}
				{/each}
			</ul>
			{#if variant === 'drawer' && list}{@render list()}{/if}
			{#if variant === 'drawer' && recentPlaces.length}
				<p class="sc-nav-head">{m['creator.recent']()}</p>
				<ul data-testid="nav-recent">
					{#each recentPlaces as p (p.href)}
						<li><a class="sc-nav-item" href={p.href} data-drawer-pick><span class="ell">{p.trail.slice(0, 2).join(' · ')}</span></a></li>
					{/each}
				</ul>
			{/if}
		</div>
		<div class="foot">
			{#if variant === 'drawer'}
				{#if foot}<div class="tools">{@render foot()}</div>{/if}
			{:else}
				<a class="sc-nav-item" href="/notifications" title={accountLabel}><Icon name="user" /><span class="ell">{account}</span></a>
				<div class="fold">
					<button type="button" class="sq quiet" onclick={() => onfold?.()} aria-label={m['creator.fold_menu']()} title="{m['creator.fold_menu']()} (Ctrl + \)" aria-expanded="true" aria-controls="creator-nav" data-testid="menu-fold"><Icon name="chevrons-left" /></button>
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.menu,
	.rail {
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		min-height: 100%;
	}
	/* the drawer: the list scrolls, the foot stays at the bottom */
	.menu.drawer {
		flex: 1 1 auto;
		min-height: 0;
	}
	.drawer .body {
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
		overscroll-behavior: contain;
	}
	.areas {
		padding-top: var(--space-05);
	}
	.sc-nav-item {
		gap: var(--space-075);
		white-space: nowrap;
	}
	/* a toon under Toons: its avatar where the areas have their icon */
	.sub {
		padding-left: calc(var(--space-1) + var(--space-025));
	}
	.ell {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	/* the dashboard's count: a small warning dot with the number */
	.badge {
		margin-left: auto;
		min-width: 1rem;
		padding: 0 var(--space-025);
		border-radius: 0.5rem;
		background: var(--state-warning);
		color: var(--ink);
		font-size: var(--type-caption-size);
		line-height: 1rem;
		text-align: center;
	}
	.foot {
		margin-top: auto;
		border-top: var(--border-width-regular) solid var(--ink);
	}
	.fold {
		display: flex;
		justify-content: flex-end;
		padding: 0 var(--space-05) var(--space-025);
	}
	.drawer .foot .tools {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding: 0 var(--space-05);
		min-height: var(--tool-bar-height);
	}
	/* the bar's icon squares (CREATOR-NAV.md §2: borderless, hover = the nav outline, on = inverted) */
	.sq {
		position: relative;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		min-width: var(--control-height);
		min-height: var(--control-height);
		padding: 0;
		border: 0;
		border-radius: var(--radius-none);
		background: transparent;
		color: var(--ink);
	}
	.sq:hover {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.sq:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	/* the fold / unfold: quiet (owner 2026-10-20: fold buttons took a lot of visual focus) */
	.sq.quiet {
		color: var(--text-muted);
	}
	.sq.quiet:hover {
		color: var(--ink);
	}
	.sq[aria-current='page'] {
		background: var(--ink);
		color: var(--paper);
	}
	.rail .sq {
		width: 100%;
		min-height: var(--tool-bar-height);
	}
	.rail .badge {
		position: absolute;
		top: var(--space-025);
		right: var(--space-025);
		margin: 0;
	}
	.rail .foot {
		display: flex;
		flex-direction: column;
	}
</style>
