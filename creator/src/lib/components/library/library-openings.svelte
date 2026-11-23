<script lang="ts">
	import { goto } from '$app/navigation';
	import { Icon } from '@scutoons/ui';
	import type { LibraryKind } from '@scutoons/media';
	import { hrefOf } from '$lib/shell/address.ts';
	import { openings } from '$lib/shell/openings.svelte.ts';
	import type { Place } from '$lib/shell/places.svelte.ts';

	/**
	 * The Library's "Items to open": the toons with their seasons and chapters and the areas, then every kind the core
	 * opens in a workspace with its rows (a press opens it as a tab), the file kinds that open in their asset details
	 * (a press shows the Library's tiles of that kind), and the kinds without a workspace yet with the core's sentence.
	 */
	let { places, only = null, query, toon, onkind, onnote }: { places: Place[]; only?: string | null; query: string; toon: string | null; onkind: (kind: LibraryKind) => void; onnote: (text: string) => void } = $props();

	$effect(() => void openings.load());

	const groups = $derived(openings.groups(query, toon).filter((g) => !only || g.kind.kind === only));

	function open(address: string) {
		const answer = openings.open(address);
		if (!answer.ok) return onnote(answer.why);
		const href = hrefOf(answer.address);
		if (href) goto(href);
		else onnote(`${answer.address}: no page for it yet`);
	}
</script>

<ul class="places" data-testid="library-openings">
	{#each places as p (p.address)}
		<li><a href={p.href} title={p.address}><Icon name={p.icon} size="1rem" /><span class="name">{p.label}</span>{#if p.hint && p.group !== 'toon'}<span class="hint">{p.hint}</span>{/if}</a></li>
	{/each}
	{#each groups as g (g.kind.kind)}
		<li class="head" data-testid="opens-{g.kind.kind}"><span class="name">{g.kind.words}</span><span class="hint">{g.kind.opens}</span></li>
		{#each g.rows as p (p.address)}
			<li><button type="button" data-control-exempt="list row" title={p.address} onclick={() => open(p.address)}><Icon name={p.icon} size="1rem" /><span class="name">{p.label}</span></button></li>
		{/each}
	{/each}
	{#if !only && openings.files().length}
		<li class="head" aria-hidden="true"></li>
		{#each openings.files() as k (k.kind)}
			<li><button type="button" data-control-exempt="list row" onclick={() => onkind(k.kind as LibraryKind)} data-testid="opens-{k.kind}"><Icon name="image" size="1rem" /><span class="name">{k.words}</span><span class="hint">{k.opens}</span></button></li>
		{/each}
	{/if}
</ul>
{#if !only && openings.closed().length}
	<ul class="closed" data-testid="library-closed">
		{#each openings.closed() as k (k.kind)}
			<li class="sc-empty" data-testid="closed-{k.kind}">{k.why}</li>
		{/each}
	</ul>
{/if}

<style>
	.places,
	.closed {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.closed {
		margin-top: 0.5rem;
	}
	.places a,
	.places button {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		min-height: var(--editor-tree-row); /* control-ok: a list row, as tall as the tree's rows */
		padding: 0 0.3rem;
		border: 0;
		border-radius: var(--editor-radius);
		background: none;
		color: var(--tool-text);
		font: inherit;
		text-align: left;
		text-decoration: none;
		text-shadow: var(--tool-halo);
		cursor: pointer;
	}
	.places a:hover,
	.places button:hover {
		background: var(--tool-hover);
	}
	.head {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
		margin-top: 0.5rem;
		padding: 0 0.3rem;
		color: var(--tool-muted);
		font-size: var(--editor-small);
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
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
</style>
