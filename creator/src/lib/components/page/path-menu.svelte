<script lang="ts">
	import { page } from '$app/state';
	import { Icon, clickOutside } from '@scutoons/ui';
	import type { Crumb } from '$lib/nav.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The path on phones (owner 2026-10-20: "something that shows the current scene's name, and clicking on it you get
	 * the full path underneath like a dropdown"): one text button with the place's name – in the chapter editor the
	 * group (scene), not the slice (the editor scrolls through a group's slices) – that opens the whole path below it,
	 * one level per row, indented, the page marked. A pick, Esc or a tap outside closes it.
	 */
	let { items, current }: { items: Crumb[]; current: Crumb } = $props();

	let open = $state(false);
	$effect(() => {
		void page.url.href;
		open = false;
	});
	const last = $derived(items.length - 1);
</script>

<div class="path-menu" {@attach clickOutside(() => (open = false))}>
	<button type="button" class="title" aria-haspopup="menu" aria-expanded={open} aria-label="{m['creator.path_menu']()}: {current.label}" onclick={() => (open = !open)} data-testid="path-menu">
		<span class="ell">{current.label}</span>
		<Icon name="chevron-down" size="1rem" />
	</button>
	{#if open}
		<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
		<ul class="menu" role="menu" aria-label={m['creator.path_menu']()} onclick={() => (open = false)} data-testid="path-menu-list">
			{#each items as c, i (i)}
				<li role="none">
					<a role="menuitem" href={c.href} style:padding-left="calc(var(--space-1) + {i * 0.75}rem)" aria-current={i === last ? 'page' : undefined}>{c.label}</a>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.path-menu {
		position: relative;
		flex: 1 1 auto;
		min-width: 0;
		padding-left: var(--space-025);
	}
	/* a text control without a frame (the bar's squares are borderless too) */
	.title {
		display: inline-flex;
		align-items: center;
		gap: var(--space-025);
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
	/* small, so a scene's name fits beside the editor's ⚠ · Save · Publish · ••• · ☰ */
	.title .ell {
		font-size: var(--type-caption-size);
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
		box-sizing: border-box;
		min-height: var(--control-height);
		padding-right: var(--space-1);
		color: var(--text-muted);
		text-decoration: none;
		overflow-wrap: anywhere;
	}
	.menu a[aria-current='page'] {
		color: var(--ink);
	}
	.menu a:hover {
		color: var(--ink);
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
</style>
