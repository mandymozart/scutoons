<script lang="ts">
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { FILE_ORIGINS, type LibraryEntry } from '@scutoons/api';
	import { CloseButton } from '@scutoons/ui';
	import { Button, Chip } from '@scutoons/ui/editor';
	import type { AssetFamily } from '@scutoons/media';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { library } from '$lib/editor/library.svelte.ts';
	import { shelf } from '$lib/editor/library-shelf.svelte.ts';
	import { findIn, placeAvailable, seasonToon, sortEntries, type PlaceContext, type ShelfPlace } from '$lib/editor/library-list.ts';
	import { deleteFamilies, downloadFamilies, renameFamily } from '$lib/editor/library-actions.ts';
	import { hrefOf } from '$lib/shell/address.ts';
	import { m } from '$lib/paraglide/messages.js';
	import AssetDetails from '$lib/components/library/asset-details.svelte';
	import LibraryFind from '$lib/components/library/library-find.svelte';
	import LibraryGrid from '$lib/components/library/library-grid.svelte';
	import LibraryMenu from '$lib/components/library/library-menu.svelte';
	import LibraryPlaces from '$lib/components/library/library-places.svelte';
	import LibraryStep from '$lib/components/library/library-step.svelte';
	import LibraryToolbar from '$lib/components/library/library-toolbar.svelte';

	/**
	 * The Library (lap2 C12, C12b, F1; LIBRARY.md): the server's one answer – things with their type beside files with
	 * their kind, one list. The places as chips, Find (Ctrl L), Upload or files dropped anywhere on it, the type, Grouping
	 * and Sort; a click selects (Ctrl / Shift several), a double click steps into a thing or uses a file; F2 renames,
	 * Delete deletes a file, right-click or ⋯ has every action. Taken full (⤢) the places stand at the left as rows. A
	 * slot's pick is the same Library on the overlay level (library-picker.svelte).
	 */
	const nav = getCreatorNav();

	let root = $state<HTMLElement>();
	let search: HTMLInputElement | null = $state(null);
	let fileInput: HTMLInputElement | null = $state(null);
	let replaceInput: HTMLInputElement | null = $state(null);
	let replacing: AssetFamily | null = null;
	let note = $state('');
	let dropping = $state(false);
	let menu = $state<{ x: number; y: number } | null>(null);

	/** the toon: the front item's, else the one the server says the open thing belongs to */
	const toon = $derived(library.toon ?? shelf.toonOf(library.address));
	const ctx = $derived<PlaceContext>({ toon, open: library.address, season: shelf.seasonFor(toon, library.address) });
	const place = $derived(shelf.place(ctx));
	const query = $derived(shelf.queryFor(ctx));
	const shown = $derived(sortEntries(findIn(shelf.entries, shelf.query), shelf.sort));
	const chips = $derived((['item', 'cast', 'toon', 'all', 'unused', 'recent'] as const).filter((p) => placeAvailable(p, ctx)));
	const placeLabel = (p: ShelfPlace) => m[`library.place_${p}`]();
	/** the crumb's first part: the place, a cast with its toon and season (`Cat Pope · Awakening · cast`) */
	const crumbPlace = $derived.by(() => {
		const season = place === 'cast' && ctx.season && toon ? shelf.toons[seasonToon(ctx.season)]?.seasons.find((s) => s.address === ctx.season) : null;
		return season ? m['library.cast_of']({ toon: nav.toonName(seasonToon(season.address)), season: season.name }) : placeLabel(place);
	});

	// the registry's files for the actions and the fit: once, and never again by itself after a failure (#881)
	$effect(() => untrack(() => void library.load()));
	$effect(() => {
		if (toon) shelf.loadToon(toon);
	});
	let seen = library.changed;
	$effect(() => {
		const q = query;
		const changed = library.changed;
		untrack(() => {
			void shelf.show(q, changed !== seen);
			seen = changed;
		});
	});

	const typing = (e: KeyboardEvent) => e.target instanceof HTMLElement && !!e.target.closest('input, textarea, select, [contenteditable]');

	function keys(event: KeyboardEvent) {
		if (library.pick) return;
		if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'l') {
			event.preventDefault();
			search?.focus();
			search?.select();
		} else if (!root?.contains(document.activeElement) || typing(event)) return;
		else if (event.key === 'F2' && library.selectedFamily) (event.preventDefault(), renameFamily(library.selectedFamily));
		else if (event.key === 'Delete' && library.chosenFamilies.length) (event.preventDefault(), deleteFamilies(library.chosenFamilies));
	}

	async function upload(files: File[]) {
		note = '';
		for (const file of files) {
			try {
				const { converted } = await library.upload(file);
				if (converted) note = m['library.converted']({ name: file.name, path: converted });
			} catch (e) {
				note = m['library.upload_failed']({ reason: e instanceof Error ? e.message : String(e) });
			}
		}
	}

	async function replace(file: File) {
		if (!replacing) return;
		note = '';
		try {
			await library.replace(replacing, file);
		} catch (e) {
			note = m['library.upload_failed']({ reason: e instanceof Error ? e.message : String(e) });
		}
	}

	/** a thing opens in its workspace by the core's open rule; a link to what is no entry (a season) too */
	async function openAddress(address: string) {
		note = '';
		const answer = (await library.opener()).open(address);
		if (!answer.ok) return (note = answer.why);
		const href = hrefOf(answer.address);
		if (href) void goto(href);
		else note = m['library.no_page']({ address: answer.address });
	}

	/** a double click: a thing is stepped into, a file used */
	function open(entry: LibraryEntry) {
		if (entry.type) return shelf.stepInto(entry.address);
		const family = library.byAddress.get(entry.address);
		if (family) library.use(family);
	}

	const hasFiles = (e: DragEvent) => !!e.dataTransfer?.types.includes('Files');
</script>

<svelte:window onkeydown={keys} />

<div
	bind:this={root}
	class="library"
	class:dropping
	data-testid="library"
	role="region"
	aria-label={m['library.title']()}
	ondragover={(e) => {
		if (!hasFiles(e)) return;
		e.preventDefault();
		dropping = true;
	}}
	ondragleave={(e) => {
		if (!root?.contains(e.relatedTarget as Node)) dropping = false;
	}}
	ondrop={(e) => {
		if (!hasFiles(e)) return;
		e.preventDefault();
		dropping = false;
		upload([...(e.dataTransfer?.files ?? [])]);
	}}
>
	<div class="side"><LibraryPlaces {ctx} {place} /></div>
	<div class="main">
		<div class="head">
			<div class="chips" role="group" aria-label={m['library.places']()}>
				{#each chips as p (p)}
					<Chip label={placeLabel(p)} on={place === p} onclick={() => shelf.setPlace(p, library.address)} testid="place-chip-{p}" />
				{/each}
				<span class="from" title={m['library.from']()}>
					{#each FILE_ORIGINS as o (o)}
						<Chip label={m[`library.from_${o}`]()} on={shelf.from === o} onclick={() => (shelf.from = shelf.from === o ? null : o)} testid="from-chip-{o}" />
					{/each}
				</span>
			</div>
			<div class="row">
				<LibraryFind bind:value={shelf.query} bind:input={search} testid="library-search" />
				<Button variant="primary" icon="upload" title={m['library.upload_title']()} onclick={() => fileInput?.click()} data-testid="library-upload">{m['library.upload']()}</Button>
				<input bind:this={fileInput} type="file" multiple hidden onchange={(e) => (upload([...(e.currentTarget.files ?? [])]), (e.currentTarget.value = ''))} />
				<input bind:this={replaceInput} type="file" hidden onchange={(e) => { const f = e.currentTarget.files?.[0]; if (f) replace(f); e.currentTarget.value = ''; }} />
			</div>
			<LibraryToolbar />
			{#if library.chosen.length > 1}
				<div class="selection" data-testid="library-selection">
					<span>{m['library.n_selected']({ count: library.chosen.length })}</span>
					{#if library.chosenFamilies.length}
						<Button variant="quiet" icon="download" onclick={() => downloadFamilies(library.chosenFamilies)}>{m['library.download']()}</Button>
						<Button variant="quiet" destroy icon="trash-2" onclick={() => deleteFamilies(library.chosenFamilies)}>{m['library.delete']()}</Button>
					{/if}
					<CloseButton label={m['library.clear_selection']()} onclick={() => library.clearSelection()} />
				</div>
			{/if}
			{#if note}<p class="sc-empty note">{note}</p>{/if}
		</div>

		<div class="body">
			{#if shelf.trail.length}
				<LibraryStep place={crumbPlace} tile={library.tile} onopen={openAddress} onmenu={(x, y) => (menu = { x, y })} />
			{:else if shelf.status === 'error'}
				<p class="sc-empty">{m['library.load_failed']({ reason: shelf.error })}</p>
			{:else if shelf.status === 'ready' && !shown.length}
				{#if shelf.query.trim()}<p class="sc-empty">{m['library.none_found']()}</p>{/if}
			{:else}
				<LibraryGrid entries={shown} tile={library.tile} list={library.list} grouped={shelf.grouping === 'type'} label={m['library.title']()} onopen={open} onmenu={(x, y) => (menu = { x, y })} />
			{/if}
		</div>
	</div>

	{#if library.details && library.selectedFamily && library.chosen.length === 1}
		<AssetDetails family={library.selectedFamily} drawer onreplace={(file) => ((replacing = library.selectedFamily), replace(file))} onclose={() => (library.details = false)} />
	{/if}
	{#if dropping}<div class="drop" aria-hidden="true">{m['library.drop_here']()}</div>{/if}
</div>

{#if menu}
	<LibraryMenu x={menu.x} y={menu.y} onclose={() => (menu = null)} onreplace={(f) => ((replacing = f), replaceInput?.click())} />
{/if}

<style>
	.library {
		position: relative;
		container: library / inline-size;
		display: flex;
		min-height: 100%;
		padding: 0 var(--editor-pad);
	}
	.side {
		display: none;
	}
	@container library (min-width: 52rem) {
		.side {
			display: flex;
			flex: 0 0 14rem;
			margin-right: var(--editor-pad);
		}
		.side > :global(*) {
			flex: 1 1 auto;
		}
	}
	.main {
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.head {
		position: sticky;
		z-index: 1;
		top: 0;
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
		padding-bottom: 0.6rem;
	}
	.body {
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		padding-bottom: var(--editor-pad);
	}
	.selection {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.3rem 0.5rem;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: var(--editor-radius);
		font-size: var(--editor-small);
	}
	.selection > span {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.chips,
	.from {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.25rem;
	}
	.from {
		padding-left: 0.35rem;
		border-left: var(--editor-edge) solid var(--tool-line);
	}
	.row {
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}
	.note {
		margin: 0;
		color: var(--tool-warn);
	}
	.drop {
		position: absolute;
		inset: 0;
		z-index: 2;
		display: grid;
		place-items: center;
		border: var(--editor-edge) dashed var(--tool-text);
		border-radius: var(--editor-radius);
		background: color-mix(in srgb, var(--tool-popover) 80%, transparent);
		text-shadow: var(--tool-halo);
		color: var(--tool-text);
		pointer-events: none;
	}
</style>
