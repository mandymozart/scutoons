<script lang="ts">
	import { untrack } from 'svelte';
	import { FILE_ORIGINS, type LibraryEntry } from '@scutoons/api';
	import { Button, Chip, Overlay } from '@scutoons/ui/editor';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { isThingPick, library } from '$lib/editor/library.svelte.ts';
	import { namesLocale, pickShelf as shelf, provideShelf } from '$lib/editor/library-shelf.svelte.ts';
	import { pickFirst, pickStepped, pressEntry, takesThing } from '$lib/editor/library-pick.svelte.ts';
	import { PICK_PLACES, findIn, firstPlace, pickChoices, pickGroups, placeAvailable, placeQuery, seasonToon, sortEntries, type PickFit, type PlaceContext, type ShelfPlace } from '$lib/editor/library-list.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LibraryFind from '$lib/components/library/library-find.svelte';
	import LibraryGrid from '$lib/components/library/library-grid.svelte';
	import LibraryStep from '$lib/components/library/library-step.svelte';
	import LibraryToolbar from '$lib/components/library/library-toolbar.svelte';

	/**
	 * The one picker (lap2 C12c): the Library opened from a slot, on the overlay level (RULES H15). "Pick for" and the
	 * slot, only what the slot takes, what this thing already has first (This item, a character's season cast) and
	 * everyone else after; the places as chips, Find (Ctrl L), the type within what fits. A press picks – a file that
	 * fits, a thing of the slot's type – fills the slot and closes; a slot that wants a level (a speaker's persona) steps
	 * into the thing first. Esc or Cancel ends it.
	 */
	provideShelf(shelf);
	const nav = getCreatorNav();

	let search: HTMLInputElement | null = $state(null);
	let fileInput: HTMLInputElement | null = $state(null);
	let note = $state('');

	// the request stays readable while the picker closes
	let last = library.pick!;
	const request = $derived((last = library.pick ?? last));
	const things = $derived(isThingPick(request));
	const fit = $derived<PickFit>(isThingPick(request) ? { types: request.types } : { kinds: request.kinds, types: request.things?.types });
	const only = $derived.by(() => {
		const c = pickChoices(fit);
		return c.length === 2 ? [c[1]] : c;
	});
	const toon = $derived(library.toon ?? shelf.toonOf(library.address));
	const ctx = $derived<PlaceContext>({ toon, open: library.address, season: shelf.seasonFor(toon, library.address) });
	const place = $derived(shelf.place(ctx));
	const from = $derived(things ? null : shelf.from);
	const query = $derived(placeQuery(place, ctx, shelf.choice, from, fit, namesLocale()));
	const first = $derived(place === 'all' ? firstPlace(request.first, ctx) : null);
	const firstQuery = $derived(first ? placeQuery(first, ctx, shelf.choice, from, fit, namesLocale()) : null);
	// a cast lists characters and their files; Unused lists files
	const chips = $derived(
		PICK_PLACES.filter((p) => placeAvailable(p, ctx) && (p !== 'unused' || !things) && (p !== 'cast' || !isThingPick(request) || request.types.includes('character')))
	);

	const keep = (e: LibraryEntry) => !things || takesThing(e);
	const order = (list: readonly LibraryEntry[]) => sortEntries(findIn(list, shelf.query), shelf.sort);
	const groups = $derived(pickGroups(order(pickFirst.entries), order(shelf.entries), keep));
	const placeLabel = (p: ShelfPlace) => m[`library.place_${p}`]();
	const seasonName = (season: string | null) => (season ? shelf.toons[seasonToon(season)]?.seasons.find((s) => s.address === season)?.name : null);
	const castLabel = $derived.by(() => {
		const name = seasonName(ctx.season);
		return name && toon ? m['library.cast_of']({ toon: nav.toonName(toon), season: name }) : placeLabel('cast');
	});
	const firstLabel = $derived(first === 'cast' ? castLabel : first === 'toon' && toon ? m['library.place_toon_named']({ toon: nav.toonName(toon) }) : first ? placeLabel(first) : '');
	const crumbPlace = $derived(place === 'cast' ? castLabel : placeLabel(place));
	const create = $derived(isThingPick(request) && request.create && shelf.query.trim() ? request.create : null);
	/** a file slot that also takes a new thing (a shot's new generator): beside Upload */
	const fileCreate = $derived(isThingPick(request) ? null : (request?.things?.create ?? null));
	/** what the slot holds, as the list names it (a file: its family's address, once the registry is read) */
	const chosen = $derived((isThingPick(request) ? request.value : request.value && library.addressOf(request.value)) ?? null);
	const stepPickable = $derived(!!shelf.step && takesThing(shelf.step.at));

	// a new slot asks: Everything, the slot's type, nothing typed, nothing stepped into
	$effect(() => {
		void request;
		untrack(() => {
			shelf.setPlace('all', library.address);
			shelf.choice = only.length === 1 ? only[0] : 'all';
			shelf.query = '';
			shelf.from = null;
			note = '';
			void library.load();
		});
	});
	$effect(() => {
		if (toon) shelf.loadToon(toon);
	});
	$effect(() => {
		const q = query;
		untrack(() => void shelf.show(q));
	});
	$effect(() => {
		const q = firstQuery;
		untrack(() => void pickFirst.show(q));
	});

	function press(entry: LibraryEntry) {
		note = pressEntry(entry);
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

	function keys(event: KeyboardEvent) {
		if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== 'l') return;
		event.preventDefault();
		search?.focus();
		search?.select();
	}

	function made() {
		const name = shelf.query.trim();
		const make = create ?? fileCreate;
		library.cancelPick();
		make?.run(name);
	}
</script>

<svelte:window onkeydown={keys} />

<Overlay place="center" label={m['library.pick_for_slot']({ slot: request.label })} onclose={() => library.cancelPick()} data-testid="library-picker">
	<section class="box">
		<header class="head">
			<h2>{m['library.title']()}</h2>
		</header>
		<div class="pick">
			<span class="for">{m['library.pick_for']()}</span>
			<Chip label={request.label} on testid="library-pick-slot" />
			<span class="sp"></span>
			<Button title={m['library.cancel_title']()} onclick={() => library.cancelPick()} data-testid="library-pick-cancel">{m['library.cancel']()}</Button>
		</div>
		<div class="chips" role="group" aria-label={m['library.places']()}>
			{#each chips as p (p)}
				<Chip label={placeLabel(p)} on={place === p} onclick={() => shelf.setPlace(p, library.address)} testid="pick-place-{p}" />
			{/each}
			{#if !things}
				<span class="from" title={m['library.from']()}>
					{#each FILE_ORIGINS as o (o)}
						<Chip label={m[`library.from_${o}`]()} on={shelf.from === o} onclick={() => (shelf.from = shelf.from === o ? null : o)} />
					{/each}
				</span>
			{/if}
		</div>
		<div class="row">
			<LibraryFind bind:value={shelf.query} bind:input={search} autofocus testid="library-pick-search" />
			{#if fileCreate}<Button icon="zap" onclick={made} data-testid="library-pick-new">{fileCreate.label(shelf.query.trim())}</Button>{/if}
			{#if !things}
				<Button variant="primary" icon="upload" title={m['library.upload_title']()} onclick={() => fileInput?.click()}>{m['library.upload']()}</Button>
				<input bind:this={fileInput} type="file" multiple hidden onchange={(e) => (upload([...(e.currentTarget.files ?? [])]), (e.currentTarget.value = ''))} />
			{/if}
		</div>
		<LibraryToolbar {only} />
		{#if note}<p class="sc-empty note">{note}</p>{/if}

		<div class="body">
			{#if shelf.trail.length}
				<LibraryStep
					place={crumbPlace}
					tile={library.tile}
					onopen={() => undefined}
					pick={{ depth: isThingPick(request) ? request.depth : undefined, thing: stepPickable ? pickStepped : undefined, entry: (e) => (note = pressEntry(e, true)), files: !things }}
				/>
			{:else if shelf.status === 'error'}
				<p class="sc-empty">{m['library.load_failed']({ reason: shelf.error })}</p>
			{:else}
				{#if groups.first.length}
					<h3 class="group">{firstLabel}</h3>
					<LibraryGrid entries={groups.first} tile={library.tile} list={library.list} grouped={shelf.grouping === 'type'} label={firstLabel} {chosen} onpress={press} onopen={press} />
					{#if groups.rest.length}<h3 class="group">{m['library.pick_else']()}</h3>{/if}
				{/if}
				<LibraryGrid entries={groups.rest} tile={library.tile} list={library.list} grouped={shelf.grouping === 'type'} label={m['library.title']()} {chosen} onpress={press} onopen={press} />
				{#if shelf.status === 'ready' && !groups.first.length && !groups.rest.length && shelf.query.trim() && !create}<p class="sc-empty">{m['library.none_found']()}</p>{/if}
				{#if create}
					<Button variant="quiet" icon="plus" onclick={made} data-testid="library-pick-create">{create.label(shelf.query.trim())}</Button>
				{/if}
			{/if}
		</div>
	</section>
</Overlay>

<style>
	/* a box of the overlay level (kit/tokens.toml [component.overlay.box]): see-through, its line, the halo on its words */
	.box {
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
		box-sizing: border-box;
		width: min(44rem, calc(100vw - 2 * var(--kit-margin)));
		height: min(48rem, calc(100vh - 2 * var(--kit-margin)));
		padding: 0 var(--editor-pad) var(--editor-pad);
		border: var(--editor-edge) solid var(--kit-overlay-box-border);
		border-radius: var(--kit-radius-box);
		background: var(--kit-overlay-box-ground);
		text-shadow: var(--tool-halo);
	}
	.head {
		display: flex;
		align-items: center;
		min-height: var(--kit-bar);
	}
	/* the box's name, as a panel's tab says it */
	.head h2 {
		margin: 0;
		font-size: inherit;
		font-weight: inherit;
		letter-spacing: normal;
		text-transform: none;
	}
	.pick,
	.chips,
	.from,
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.35rem;
	}
	.for {
		color: var(--tool-text);
	}
	.sp {
		flex: 1 1 auto;
	}
	.from {
		padding-left: 0.35rem;
		border-left: var(--editor-edge) solid var(--tool-line);
	}
	.row {
		flex-wrap: nowrap;
	}
	.note {
		margin: 0;
		color: var(--tool-warn);
	}
	.body {
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: 0.6rem;
		min-height: 0;
		overflow: auto;
		padding-top: 0.2rem;
	}
	.body > :global(.kit-button) {
		align-self: flex-start;
	}
	.group {
		margin: 0.2rem 0 0;
		color: var(--tool-muted);
		font-size: var(--editor-label);
		font-weight: 400;
		letter-spacing: var(--editor-label-spacing, 0.08em);
		text-transform: uppercase;
	}
</style>
