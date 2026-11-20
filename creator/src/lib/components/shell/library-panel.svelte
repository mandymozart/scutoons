<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import type { WorkspaceKind } from '$lib/shell/address.ts';
	import { matches, places, type Place } from '$lib/shell/places.svelte.ts';

	/**
	 * The Library, first cut (research/redesign/PLAN.md §3): what the existing APIs list – the toons with their seasons and
	 * chapters, the areas, the catalog's kinds – searchable by name or address; a toon's workspace pre-filters it to that
	 * toon, "All" widens it. The asset manager (assets, families, use counts) is its own ticket.
	 */
	let { kind, toon = null }: { kind: WorkspaceKind | 'home'; toon?: string | null } = $props();

	const nav = getCreatorNav();
	$effect(() => places.load(nav.toons));

	let query = $state('');
	let scope = $state<'here' | 'all'>('here');
	const scoped = $derived(!!toon && (kind === 'toon' || kind === 'chapter' || kind === 'storyboard'));
	const list = $derived.by<Place[]>(() => {
		const all = [...places.toons(nav.toons, (id) => nav.toonName(id), toon), ...places.areas()];
		const here = scoped && scope === 'here' ? all.filter((p) => p.toon === toon) : all;
		return here.filter((p) => matches(p, query));
	});
</script>

<div class="library" data-testid="library">
	<input class="find" type="search" bind:value={query} placeholder={m['shell.find_placeholder']()} aria-label={m['shell.find']()} />
	{#if scoped}
		<div class="chips">
			<button type="button" class="chip" aria-pressed={scope === 'here'} onclick={() => (scope = 'here')}>{nav.toonName(toon!)}</button>
			<button type="button" class="chip" aria-pressed={scope === 'all'} onclick={() => (scope = 'all')}>{m['shell.all']()}</button>
		</div>
	{/if}
	<ul class="rows">
		{#each list as p (p.address)}
			<li>
				<a href={p.href} title={p.address}>
					<Icon name={p.icon} size="1rem" />
					<span class="name">{p.label}</span>
					{#if p.hint && p.group !== 'toon'}<span class="hint">{p.hint}</span>{/if}
				</a>
			</li>
		{/each}
	</ul>
</div>

<style>
	.library {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		padding: 0 var(--editor-pad) var(--editor-pad);
	}
	.find {
		width: 100%;
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
	.chip {
		min-height: 1.7rem;
		padding: 0 0.6rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: 1rem;
		background: var(--editor-control);
		color: var(--editor-text);
		font-size: var(--editor-small);
	}
	.chip[aria-pressed='true'] {
		background: var(--editor-pressed);
		color: var(--editor-pressed-text);
	}
	.rows {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.rows a {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-height: var(--editor-tree-row);
		padding: 0 0.3rem;
		border-radius: var(--editor-radius);
		color: var(--editor-text);
		text-decoration: none;
		text-shadow: var(--editor-halo);
	}
	.rows a:hover {
		background: var(--editor-hover);
	}
	.name {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.hint {
		flex: none;
		color: var(--editor-muted);
		font-size: var(--editor-small);
	}
</style>
