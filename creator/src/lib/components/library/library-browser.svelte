<script lang="ts">
	import { CloseButton, Icon } from '@scutoons/ui';
	import { LIBRARY_KINDS, beforeKinds, fits, groupByKind, kindCounts, sortFamilies, type AssetFamily } from '@scutoons/media';
	import { library } from '$lib/editor/library.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import AssetTile from '$lib/components/library/asset-tile.svelte';
	import AssetDetails from '$lib/components/library/asset-details.svelte';
	import LibraryPlaces from '$lib/components/library/library-places.svelte';

	/**
	 * The Library over the stage (⤢, research/redesign/library.html ?w=browser): places at the left, the grid grouped by
	 * kind, the details drawer at the right. Esc closes it back to the panel.
	 */
	let fileInput: HTMLInputElement | null = $state(null);
	let note = $state('');

	const filter = $derived({ scope: library.scope, context: library.context, toon: library.toon, query: library.query, todo: library.todo, folder: library.folder, origin: library.origin });
	const scoped = $derived(beforeKinds(library.families, filter));
	const kinds = $derived(kindCounts(scoped));
	const groups = $derived(groupByKind(sortFamilies(library.kind === 'all' || library.kind === 'entities' ? scoped : scoped.filter((f) => f.kind === library.kind), library.sort)));
	const total = $derived(groups.reduce((n, g) => n + g.items.length, 0));

	function keys(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			library.browser = false;
		}
	}

	async function run(work: () => Promise<unknown>) {
		note = '';
		try {
			await work();
		} catch (e) {
			note = m['library.upload_failed']({ reason: e instanceof Error ? e.message : String(e) });
		}
	}
	const replace = (f: AssetFamily, file: File) => run(() => library.replace(f, file));
</script>

<svelte:window onkeydown={keys} />

<div class="browser" role="dialog" aria-label={m['library.title']()} data-testid="library-browser">
	<header>
		<span class="lbl">{m['library.title']()}</span>
		<input class="find" type="search" bind:value={library.query} placeholder={m['library.search']()} aria-label={m['library.search']()} />
		<div class="chips">
			<button type="button" class="chip" data-control-exempt="filter chip" aria-pressed={library.kind === 'all' || library.kind === 'entities'} onclick={() => (library.kind = 'all')}>{m['library.kind_all']()}</button>
			{#each LIBRARY_KINDS as k (k)}
				{#if kinds[k]}<button type="button" class="chip" data-control-exempt="filter chip" aria-pressed={library.kind === k} onclick={() => (library.kind = k)}>{m[`library.kinds_${k}`]()}<span class="n">{kinds[k]}</span></button>{/if}
			{/each}
		</div>
		<span class="sp"></span>
		<button type="button" class="primary" onclick={() => fileInput?.click()}><Icon name="upload" size="0.9rem" /> {m['library.upload']()}</button>
		<input bind:this={fileInput} type="file" hidden onchange={(e) => { const f = e.currentTarget.files?.[0]; if (f) run(() => library.upload(f)); e.currentTarget.value = ''; }} />
		<CloseButton label={m['library.close_browser']()} onclick={() => (library.browser = false)} />
	</header>
	<div class="body" class:with-details={!!library.selectedFamily}>
		<LibraryPlaces />
		<div class="main">
			{#if note}<p class="sc-empty note">{note}</p>{/if}
			{#each groups as g (g.kind)}
				<h3>{m[`library.kinds_${g.kind}`]()} · {g.items.length}</h3>
				<div class="tiles" role="listbox" aria-label={m[`library.kinds_${g.kind}`]()}>
					{#each g.items as f (f.key)}
						<AssetTile
							family={f}
							selected={library.selected === f.key}
							dimmed={!fits(f, library.pick)}
							onselect={() => (library.selected = f.key)}
							onuse={() => (library.use(f), (library.browser = false))}
							onreplace={(file) => replace(f, file)}
							ondetails={() => (library.selected = f.key)}
						/>
					{/each}
				</div>
			{/each}
			{#if library.status === 'ready' && !total}<p class="sc-empty">{m['library.none_found']()}</p>{/if}
			{#if library.status === 'loading'}<p class="sc-empty">{m['library.loading']()}</p>{/if}
			<p class="foot">{m['library.browser_foot']({ count: total })}</p>
		</div>
		{#if library.selectedFamily}
			<AssetDetails family={library.selectedFamily} onreplace={(file) => replace(library.selectedFamily!, file)} />
		{/if}
	</div>
</div>

<style>
	.browser {
		position: fixed;
		z-index: 60;
		top: var(--shell-top, 3.6rem);
		left: var(--editor-gap, 1rem);
		right: var(--editor-gap, 1rem);
		bottom: var(--editor-gap, 1rem);
		display: flex;
		flex-direction: column;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: var(--editor-radius);
		background: var(--editor-surface);
		color: var(--editor-text);
		pointer-events: auto;
		overflow: hidden;
	}
	header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.45rem var(--editor-pad);
		border-bottom: var(--editor-edge) solid var(--editor-line);
	}
	.lbl {
		color: var(--editor-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing, 0.08em);
		text-transform: uppercase;
	}
	.find {
		width: 26rem;
		max-width: 35%;
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
	.chip {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		min-height: 1.7rem;
		padding: 0 0.6rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: 1rem;
		background: var(--editor-control);
		color: var(--editor-text);
		font-size: var(--editor-small);
		cursor: pointer;
	}
	.chip[aria-pressed='true'] {
		background: var(--editor-pressed);
		color: var(--editor-pressed-text);
	}
	.n {
		font-size: var(--editor-label);
		opacity: 0.7;
	}
	.sp {
		flex: 1 1 auto;
	}
	.primary {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		padding: 0 0.7rem;
		border: var(--editor-edge) solid var(--editor-primary-line);
		border-radius: var(--editor-radius);
		background: var(--editor-primary);
		color: var(--editor-primary-ink);
		cursor: pointer;
	}
	.body {
		flex: 1 1 auto;
		min-height: 0;
		display: grid;
		grid-template-columns: 15rem minmax(0, 1fr);
	}
	.body.with-details {
		grid-template-columns: 15rem minmax(0, 1fr) 24rem;
	}
	.main {
		overflow: auto;
		padding: 0.3rem var(--editor-pad) var(--editor-pad);
	}
	h3 {
		margin: 0.5rem 0 0.3rem;
		color: var(--editor-muted);
		font-size: var(--editor-label);
		font-weight: 400;
		letter-spacing: var(--editor-label-spacing, 0.08em);
		text-transform: uppercase;
	}
	.tiles {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(9rem, 1fr));
		gap: 0.45rem;
	}
	.foot {
		margin: 0.8rem 0 0;
		color: var(--editor-muted);
		font-size: var(--editor-small);
	}
	.note {
		color: var(--editor-warn);
	}
</style>
