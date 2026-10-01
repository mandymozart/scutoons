<script lang="ts">
	import { page } from '$app/state';
	import { ToonAvatar } from '@scutoons/ui';
	import { entities, parseEntityKey, toonLanguages, type ToonIndex } from '@scutoons/content';
	import { contentIndex } from '$lib/api.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { fileLabel } from '$lib/labels.ts';

	/**
	 * The layout's default sidebar: the children of the current level (home → toons; a toon and its cast / credits /
	 * settings → seasons + Cast + Credits + Toon settings). Levels whose children the page owns (a season's chapters,
	 * a chapter's groups and slices) are rendered by the page (`usePageNav`). Every child link carries
	 * `data-nav-child` (Alt+ArrowDown opens the first one).
	 */
	let { toons, toon }: { toons: ToonIndex[]; toon: string | null } = $props();

	let seasons = $state<number[]>([]);
	$effect(() => {
		const id = toon;
		seasons = [];
		if (!id) return;
		contentIndex(id)
			.then((index) => {
				const numbers = [...new Set(index.entities.map((e) => parseEntityKey(e.key)).filter((r) => r?.kind === 'season').map((r) => r!.number!))];
				if (id === toon) seasons = numbers.length ? numbers.sort((a, b) => a - b) : [1];
			})
			.catch(() => {
				if (id === toon) seasons = [1];
			});
	});

	const name = (t: ToonIndex) => t.i18n?.[getLocale()]?.name ?? t.i18n?.[toonLanguages(t).source]?.name ?? t.id;
	const path = $derived(page.url.pathname);
	const current = (href: string) => (path === href ? 'page' : undefined);
	/** the items are the children of the page shown (not its siblings): Alt+ArrowDown may open them */
	const childAttr = $derived((toon ? path === `/${toon}` : path === '/') ? '' : undefined);
</script>

{#if toon}
	<p class="sc-nav-head">{m['creator.seasons']()}</p>
	<ul>
		{#each seasons as n (n)}
			<li><a class="sc-nav-item" href="/{toon}/season/{n}" data-nav-child={childAttr} aria-current={path.startsWith(`/${toon}/season/${n}`) ? 'page' : undefined}>{fileLabel(entities.season(toon, n))}</a></li>
		{/each}
	</ul>
	<p class="sc-nav-head">{m['creator.meta']()}</p>
	<ul>
		<li><a class="sc-nav-item" href="/{toon}/cast" data-nav-child={childAttr} aria-current={current(`/${toon}/cast`)}>{m['creator.cast']()}</a></li>
		<li><a class="sc-nav-item" href="/{toon}/credits" data-nav-child={childAttr} aria-current={current(`/${toon}/credits`)}>{m['creator.credits']()}</a></li>
		<li><a class="sc-nav-item" href="/{toon}/script" data-nav-child={childAttr} aria-current={current(`/${toon}/script`)} data-testid="nav-script">{m['creator.script']()}</a></li>
		<li><a class="sc-nav-item" href="/{toon}/toon" data-nav-child={childAttr} aria-current={current(`/${toon}/toon`)} data-testid="nav-toon-settings">{fileLabel(entities.toonSettings(toon))}</a></li>
	</ul>
{:else}
	<p class="sc-nav-head">{m['creator.toons']()}</p>
	<ul>
		{#each toons as t (t.id)}
			<li>
				<a class="sc-nav-item toon" href="/{t.id}" data-nav-child={childAttr} data-testid="nav-toon-{t.id}">
					<ToonAvatar src={t.avatar ? asset(`${t.id}/${t.avatar}`) : null} name={name(t)} size="1.5rem" />
					<span>{name(t)}</span>
				</a>
			</li>
		{/each}
	</ul>
{/if}

<style>
	/* the entries are the tool vocabulary (tools.css: sc-nav-head, sc-nav-item) */
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.toon span {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
