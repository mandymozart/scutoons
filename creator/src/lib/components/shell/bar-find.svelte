<script lang="ts">
	import { goto } from '$app/navigation';
	import { Icon, clickOutside } from '@scutoons/ui';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { resolveTyped } from '$lib/shell/address.ts';
	import { matches, places } from '$lib/shell/places.svelte.ts';

	/** Find anything by name or address – the bar's middle on Home (the deck editor's bar has no empty middle either) */
	const nav = getCreatorNav();
	$effect(() => places.load(nav.toons));

	let query = $state('');
	let open = $state(false);
	const all = $derived([...places.toons(nav.toons, (id) => nav.toonName(id)), ...places.areas()]);
	const found = $derived(query.trim() ? all.filter((p) => matches(p, query)).slice(0, 10) : []);
	function find(event: SubmitEvent) {
		event.preventDefault();
		const href = resolveTyped(query) ?? found[0]?.href;
		if (href) {
			open = false;
			void goto(href);
		}
	}
</script>

<div class="bar-find" {@attach clickOutside(() => (open = false))} data-testid="home-find">
	<form class="find" onsubmit={find} role="search">
		<Icon name="search" size="1rem" />
		<input type="search" bind:value={query} onfocus={() => (open = true)} oninput={() => (open = true)} placeholder={m['shell.find_placeholder']()} aria-label={m['shell.find']()} data-testid="home-find-input" />
	</form>
	{#if open && found.length}
		<ul class="rows">
			{#each found as p (p.address)}
				<li><a href={p.href} onclick={() => (open = false)}><Icon name={p.icon} size="1rem" /><span class="ell">{p.label}</span><span class="addr">{p.address}</span></a></li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.bar-find {
		position: relative;
		width: min(34rem, 100%);
	}
	.find {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 0.3rem 0 0.9rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: 1.2rem;
		background: var(--editor-panel-ground);
		text-shadow: var(--editor-halo);
		color: var(--editor-muted);
	}
	.find input {
		flex: 1;
		min-width: 0;
		min-height: calc(var(--control-height) - 0.2rem);
		border: 0;
		background: transparent;
		color: var(--editor-text);
		outline: none;
	}
	.rows {
		position: absolute;
		z-index: 60;
		top: calc(100% + 0.45rem);
		left: 0;
		right: 0;
		margin: 0;
		padding: 0.3rem;
		list-style: none;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: var(--editor-radius);
		background: var(--editor-surface);
		box-shadow: 0 0.4rem 1.2rem rgb(0 0 0 / 0.4);
	}
	.rows a {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		min-height: var(--editor-tree-row);
		padding: 0 0.5rem;
		border-radius: var(--editor-radius);
		color: var(--editor-text);
		text-decoration: none;
	}
	.rows a:hover {
		background: var(--editor-hover);
	}
	.ell {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.addr {
		margin-left: auto;
		color: var(--editor-muted);
		font-family: var(--font-mono);
		font-size: var(--editor-small);
	}
</style>
