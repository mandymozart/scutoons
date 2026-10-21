<script lang="ts">
	import { page } from '$app/state';
	import { Icon, ToonAvatar, clickOutside } from '@scutoons/ui';
	import { toonName, type ToonIndex } from '@scutoons/content';
	import { asset } from '$lib/utils/cdn.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The toon picker in the phone's top row on a toon's own pages (owner 2026-10-21: "the toon selector in the top … a
	 * version which we had in the map, but without the outline – avatar and name and a tiny chevron"): the toon's avatar ·
	 * its name · a chevron; a tap lists the toons (Toons on top). Another toon opens the same tab of that toon (Cast stays
	 * Cast); from a season's page its Stories.
	 */
	let { toons, toon }: { toons: ToonIndex[]; toon: string } = $props();

	let open = $state(false);
	$effect(() => {
		void page.url.href;
		open = false;
	});
	const current = $derived(toons.find((t) => t.id === toon || t.uri === toon));
	const name = (t: ToonIndex | undefined) => (t ? toonName(t, getLocale()) : toon);
	const avatar = (t: ToonIndex | undefined) => (t?.avatar ? asset(`${t.id}/${t.avatar}`) : null);
	/** the same page of another toon: /<toon>[/cast|/credits|/script|/toon]; a season's page → the toon's Stories */
	const hrefFor = (id: string) => {
		const rest = page.url.pathname.split('/').slice(2).join('/');
		return /^(cast|credits|script|toon)$/.test(rest) ? `/${id}/${rest}` : `/${id}`;
	};
</script>

<div class="toon-switch" {@attach clickOutside(() => (open = false))}>
	<button type="button" class="title" aria-haspopup="menu" aria-expanded={open} aria-label="{m['creator.toons']()}: {name(current)}" onclick={() => (open = !open)} data-testid="toon-switch">
		<ToonAvatar src={avatar(current)} name={name(current)} size="1.75rem" />
		<span class="ell">{name(current)}</span>
		<Icon name="chevron-down" size="0.875rem" />
	</button>
	{#if open}
		<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
		<ul class="menu" role="menu" aria-label={m['creator.toons']()} onclick={() => (open = false)} data-testid="toon-switch-list">
			<li role="none"><a role="menuitem" href="/toons" class="all">{m['creator.toons']()}</a></li>
			{#each toons as t (t.id)}
				<li role="none">
					<a role="menuitem" href={hrefFor(t.id)} aria-current={t === current ? 'page' : undefined} data-testid="toon-switch-{t.id}">
						<ToonAvatar src={avatar(t)} name={name(t)} size="1.75rem" />
						<span class="ell">{name(t)}</span>
					</a>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.toon-switch {
		position: relative;
		flex: 1 1 auto;
		min-width: 0;
		padding-left: var(--space-025);
	}
	/* no frame (the map's pill had one): avatar · name · a small chevron */
	.title {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		max-width: 100%;
		min-height: var(--control-height);
		padding: 0 var(--space-025);
		border: 0;
		background: transparent;
		color: var(--ink);
	}
	.title:hover,
	.title[aria-expanded='true'] {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.ell {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.menu {
		position: absolute;
		top: calc(100% + var(--space-025));
		left: 0;
		z-index: 40;
		min-width: 16rem;
		max-width: calc(100vw - 2rem);
		margin: 0;
		padding: var(--space-025) 0;
		list-style: none;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
	}
	.menu a {
		display: flex;
		align-items: center;
		gap: var(--space-075);
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 var(--space-1);
		color: var(--ink);
		text-decoration: none;
	}
	.menu a.all {
		color: var(--text-muted);
	}
	.menu a[aria-current='page'] {
		background: var(--ink);
		color: var(--paper);
	}
	.menu a:hover:not([aria-current='page']) {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
</style>
