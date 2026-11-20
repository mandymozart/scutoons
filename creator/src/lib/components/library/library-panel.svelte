<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { LIBRARY_KINDS, LIBRARY_SORTS, beforeKinds, fits, kindCounts, scopeCounts, sortFamilies, type AssetFamily, type LibraryScope } from '@scutoons/media';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { library } from '$lib/editor/library.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { matches, places, type Place } from '$lib/shell/places.svelte.ts';
	import AssetTile from '$lib/components/library/asset-tile.svelte';
	import VersionsTray from '$lib/components/library/versions-tray.svelte';

	/**
	 * The Library panel (research/redesign/library.html ?w=panel): the workspace's context first – Toon / All / Unused
	 * widen it –, search (Ctrl L) and Upload, kinds counted within the scope, sort, tile size, tiles / list. One tile per
	 * asset family; ⧉ opens its versions; ⤢ the browser over the stage. A blank slot's pick puts the slot on top and dims
	 * what doesn't fit. "Entities" lists what opens as a tab (toons, chapters, the areas).
	 */
	const nav = getCreatorNav();
	$effect(() => places.load(nav.toons));
	$effect(() => void library.load());

	let search: HTMLInputElement | null = $state(null);
	let fileInput: HTMLInputElement | null = $state(null);
	let note = $state('');

	const ctx = $derived(library.context);
	const filter = $derived({ scope: library.scope, context: ctx, toon: library.toon, query: library.query });
	const counts = $derived(scopeCounts(library.families, ctx, library.toon));
	const scoped = $derived(beforeKinds(library.families, filter));
	const kinds = $derived(kindCounts(scoped));
	const shown = $derived(
		library.kind === 'entities' ? [] : sortFamilies(library.kind === 'all' ? scoped : scoped.filter((f) => f.kind === library.kind), library.sort)
	);
	const entities = $derived.by<Place[]>(() => {
		const all = [...places.toons(nav.toons, (id) => nav.toonName(id), library.toon), ...places.areas()];
		const here = library.scope !== 'all' && library.toon ? all.filter((p) => p.toon === library.toon) : all;
		return here.filter((p) => matches(p, library.query));
	});
	const scopes = $derived<{ id: LibraryScope; label: string; count: number }[]>([
		...(ctx && ctx.kind !== 'toon' ? [{ id: 'context' as const, label: m[`library.scope_${ctx.kind}`](), count: counts.context }] : []),
		...(library.toon ? [{ id: 'toon' as const, label: m['library.scope_toon'](), count: counts.toon }] : []),
		{ id: 'all', label: m['library.scope_all'](), count: counts.all },
		{ id: 'unused', label: m['library.scope_unused'](), count: counts.unused }
	]);
	$effect(() => {
		if (!scopes.some((s) => s.id === library.scope)) library.scope = scopes[0].id;
	});

	function keys(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'l') {
			event.preventDefault();
			search?.focus();
			search?.select();
		} else if (event.key === 'Escape' && !event.defaultPrevented && library.pick && !library.browser) library.cancelPick();
	}

	async function upload(file: File) {
		note = '';
		try {
			await library.upload(file);
		} catch (e) {
			note = m['library.upload_failed']({ reason: e instanceof Error ? e.message : String(e) });
		}
	}

	async function replace(family: AssetFamily, file: File) {
		note = '';
		try {
			await library.replace(family, file);
		} catch (e) {
			note = m['library.upload_failed']({ reason: e instanceof Error ? e.message : String(e) });
		}
	}

	function details(family: AssetFamily) {
		library.selected = family.key;
		library.browser = true;
	}
</script>

<svelte:window onkeydown={keys} />

<div class="library" class:picking={!!library.pick} data-testid="library">
	<div class="head">
	{#if library.pick}
		<div class="pick" data-testid="library-pick">
			<span>{m['library.pick_for']()} <span class="slot">{library.pick.label}</span></span>
			<button type="button" class="link" data-control-exempt="text link" onclick={() => library.cancelPick()}>{m['library.cancel_esc']()}</button>
		</div>
	{/if}
	<div class="row">
		<div class="chips" role="radiogroup" aria-label={m['library.scope']()}>
			{#each scopes as s (s.id)}
				<button type="button" class="chip" data-control-exempt="filter chip" role="radio" aria-checked={library.scope === s.id} onclick={() => (library.scope = s.id)} data-testid="scope-{s.id}">
					{s.label}<span class="n">{s.count}</span>
				</button>
			{/each}
		</div>
		<button type="button" class="round" title={m['library.open_browser']()} aria-label={m['library.open_browser']()} onclick={() => (library.browser = true)} data-testid="library-expand"><Icon name="maximize-2" size="0.9rem" /></button>
	</div>
	<div class="row">
		<input bind:this={search} class="find" type="search" bind:value={library.query} placeholder={m['library.search']()} aria-label={m['library.search']()} data-testid="library-search" />
		<button type="button" class="primary" onclick={() => fileInput?.click()} data-testid="library-upload"><Icon name="upload" size="0.9rem" /> {m['library.upload']()}</button>
		<input bind:this={fileInput} type="file" hidden onchange={(e) => { const f = e.currentTarget.files?.[0]; if (f) upload(f); e.currentTarget.value = ''; }} />
	</div>
	<div class="chips">
		<button type="button" class="chip" data-control-exempt="filter chip" aria-pressed={library.kind === 'all'} onclick={() => (library.kind = 'all')}>{m['library.kind_all']()}</button>
		{#each LIBRARY_KINDS as k (k)}
			{#if kinds[k]}
				<button type="button" class="chip" data-control-exempt="filter chip" aria-pressed={library.kind === k} onclick={() => (library.kind = k)} data-testid="kind-{k}">{m[`library.kinds_${k}`]()}<span class="n">{kinds[k]}</span></button>
			{/if}
		{/each}
		<button type="button" class="chip" data-control-exempt="filter chip" aria-pressed={library.kind === 'entities'} onclick={() => (library.kind = 'entities')} data-testid="kind-entities">{m['library.entities']()}<span class="n">{entities.length}</span></button>
	</div>
	<div class="row tools">
		<select value={library.sort} onchange={(e) => library.setView({ sort: e.currentTarget.value as (typeof LIBRARY_SORTS)[number] })} aria-label={m['library.sort']()} title={m['library.sort']()}>
			{#each LIBRARY_SORTS as s (s)}<option value={s}>{m[`library.sort_${s}`]()}</option>{/each}
		</select>
		<span class="sp"></span>
		<button type="button" class="square" aria-pressed={library.size === 's'} title={m['library.size_s']()} aria-label={m['library.size_s']()} onclick={() => library.setView({ size: 's', list: false })}><Icon name="grid" size="0.8rem" /></button>
		<button type="button" class="square" aria-pressed={library.size === 'm' && !library.list} title={m['library.size_m']()} aria-label={m['library.size_m']()} onclick={() => library.setView({ size: 'm', list: false })}><Icon name="square" size="0.9rem" /></button>
		<button type="button" class="square" aria-pressed={library.list} title={m['library.as_list']()} aria-label={m['library.as_list']()} onclick={() => library.setView({ list: !library.list })}><Icon name="list" size="0.9rem" /></button>
	</div>
	{#if note}<p class="sc-empty note">{note}</p>{/if}
	</div>

	{#if library.kind === 'entities'}
		<ul class="places">
			{#each entities as p (p.address)}
				<li><a href={p.href} title={p.address}><Icon name={p.icon} size="1rem" /><span class="name">{p.label}</span>{#if p.hint && p.group !== 'toon'}<span class="hint">{p.hint}</span>{/if}</a></li>
			{/each}
		</ul>
	{:else if library.status === 'error'}
		<p class="sc-empty">{m['library.load_failed']({ reason: library.error })}</p>
	{:else if library.status === 'ready' && !shown.length}
		<p class="sc-empty">{library.query ? m['library.none_found']() : m['library.none_here']()}</p>
	{:else}
		<div class="tiles size-{library.size}" class:list={library.list} role="listbox" aria-label={m['library.title']()} data-testid="library-tiles">
			{#each shown as f (f.key)}
				<AssetTile
					family={f}
					row={library.list}
					selected={library.selected === f.key}
					dimmed={!fits(f, library.pick)}
					trayOpen={library.tray === f.key}
					onselect={() => (library.selected = f.key)}
					onuse={() => library.use(f)}
					onreplace={(file) => replace(f, file)}
					ondetails={() => details(f)}
					ontray={() => (library.tray = library.tray === f.key ? null : f.key)}
				/>
				{#if library.tray === f.key}
					<VersionsTray family={f} onuse={(v) => library.use(f, v.path)} onclose={() => (library.tray = null)} />
				{/if}
			{/each}
		</div>
	{/if}
</div>

<style>
	.library {
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
		padding: 0 var(--editor-pad) var(--editor-pad);
	}
	.head {
		position: sticky;
		z-index: 1;
		top: 0;
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
		padding-bottom: 0.45rem;
		background: var(--editor-surface);
	}
	.pick {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.3rem 0.5rem;
		border: var(--editor-edge) solid var(--editor-primary-line);
		border-radius: var(--editor-radius);
		font-size: var(--editor-small);
	}
	.pick > span {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.slot {
		color: var(--editor-bright);
	}
	.link {
		border: 0;
		background: none;
		color: var(--editor-accent);
		cursor: pointer;
		font: inherit;
	}
	.row {
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}
	.row .chips {
		flex: 1 1 auto;
	}
	.find {
		flex: 1 1 auto;
		min-width: 0;
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
	.chip[aria-pressed='true'],
	.chip[aria-checked='true'] {
		background: var(--editor-pressed);
		color: var(--editor-pressed-text);
	}
	.n {
		font-size: var(--editor-label);
		opacity: 0.7;
	}
	.round {
		flex: none;
		width: var(--control-height);
		border: 0;
		border-radius: 50%;
		background: none;
		color: var(--editor-muted);
		display: inline-grid;
		place-items: center;
		cursor: pointer;
	}
	.primary {
		flex: none;
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
	.tools select {
		max-width: 12rem;
	}
	.sp {
		flex: 1 1 auto;
	}
	.square {
		flex: none;
		width: var(--control-height);
		border: 0;
		border-radius: var(--editor-radius);
		background: none;
		color: var(--editor-muted);
		display: inline-grid;
		place-items: center;
		cursor: pointer;
	}
	.square[aria-pressed='true'] {
		background: var(--editor-pressed);
		color: var(--editor-pressed-text);
	}
	.note {
		margin: 0;
		color: var(--editor-warn);
	}
	.tiles {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(var(--tile), 1fr));
		gap: 0.45rem;
		--tile: 7rem;
	}
	.tiles.size-s {
		--tile: 4.8rem;
	}
	.tiles.size-l {
		--tile: 10rem;
	}
	.tiles.list {
		grid-template-columns: 1fr;
		gap: 0;
	}
	.places {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.places a {
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
	.places a:hover {
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
