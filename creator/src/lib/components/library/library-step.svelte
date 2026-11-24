<script lang="ts">
	import type { LibraryEntry, LibraryLink } from '@scutoons/api';
	import { parseEntityKey } from '@scutoons/content';
	import { Icon } from '@scutoons/ui';
	import { Button, Chip, IconButton, ListRow } from '@scutoons/ui/editor';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { library } from '$lib/editor/library.svelte.ts';
	import { shelf } from '$lib/editor/library-shelf.svelte.ts';
	import { levelColumns, levelKey } from '$lib/editor/library-list.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LibraryGrid from '$lib/components/library/library-grid.svelte';

	/**
	 * A thing stepped into (lap2 C12b): the crumb at the top – the place, then every thing stepped into, back by a press –
	 * its levels as columns (a character's personas → looks → moods, a toon's seasons → chapters; each keeps its address),
	 * beside them the things and files it uses, and "used in": who uses it, each a link that opens it. Open takes the
	 * thing to its workspace.
	 */
	let { place, tile, onopen, onmenu }: { place: string; tile: number; onopen: (address: string) => void; onmenu: (x: number, y: number) => void } = $props();

	const nav = getCreatorNav();
	const step = $derived(shelf.step);
	const at = $derived(step && step.at.address === shelf.trail.at(-1) ? step : null);
	const columns = $derived(at ? levelColumns(at.levels, at.at.address, shelf.chosen) : []);
	const chosenLevels = $derived(columns.map((c, i) => c.rows.find((l) => levelKey(l) === shelf.chosen[i])).filter((l) => !!l));
	/** the deepest chosen level's picture: the file it shows comes first among the files */
	const showing = $derived(chosenLevels.at(-1)?.picture ?? null);
	/** the files beside the levels: the first few, all of them on a press ("used in" stays in sight) */
	const FEW = 12;
	let all = $state(false);
	$effect(() => {
		void shelf.trail.at(-1);
		all = false;
	});
	const beside = $derived<LibraryEntry[]>(at ? [...at.uses, ...at.files.filter((f) => showing && f.picture === showing), ...at.files.filter((f) => !showing || f.picture !== showing)] : []);

	const LEVEL_WORDS: Record<string, () => string> = {
		persona: m['library.level_persona'],
		look: m['library.level_look'],
		mood: m['library.level_mood'],
		season: m['library.level_season'],
		chapter: m['library.level_chapter']
	};

	/** a link in words: a season or a chapter with its toon first (`Cat Pope · Awakening`), the way it uses */
	function linkLabel(link: LibraryLink): string {
		const toon = link.type ? null : parseEntityKey(link.address)?.toon;
		return toon ? `${nav.toonName(toon)} · ${link.name}` : link.name;
	}

	function open(entry: LibraryEntry) {
		if (entry.type) return shelf.stepInto(entry.address, true);
		const family = library.byAddress.get(entry.address);
		if (family) library.use(family);
	}
</script>

<div class="step" data-testid="library-step">
	<nav class="crumb" aria-label={m['library.crumb']()}>
		<IconButton icon="chevron-left" label={m['library.back']()} onclick={() => shelf.backTo(shelf.trail.length - 2)} data-testid="library-back" />
		<button type="button" class="part" data-control-exempt="crumb" onclick={() => shelf.leave()}>{place}</button>
		{#each shelf.trail as address, i (address)}
			<span class="sep" aria-hidden="true">›</span>
			{#if i < shelf.trail.length - 1}
				<button type="button" class="part" data-control-exempt="crumb" onclick={() => shelf.backTo(i)}>{shelf.names[address] ?? address}</button>
			{:else if at}
				{#if at.at.picture}<img class="avatar" src={at.at.picture} alt="" />{/if}
				<button type="button" class="part here" data-control-exempt="crumb" onclick={() => (shelf.chosen = [])}>{at.at.name}</button>
				<Chip label={at.at.type ? m[`library.type_${at.at.type}`]() : m[`library.file_${at.at.kind ?? 'file'}`]()} />
			{/if}
		{/each}
		{#each chosenLevels as l, i (levelKey(l))}
			<span class="sep" aria-hidden="true">›</span>
			<button type="button" class="part" data-control-exempt="crumb" title={l.address} onclick={() => (shelf.chosen = shelf.chosen.slice(0, i + 1))}>{l.name}</button>
		{/each}
		<span class="sp"></span>
		{#if at?.at.type}<Button icon="external-link" onclick={() => onopen(at.at.address)} data-testid="library-step-open">{m['library.open']()}</Button>{/if}
	</nav>

	{#if shelf.stepStatus === 'error'}
		<p class="sc-empty">{m['library.step_failed']()}</p>
	{:else if at}
		<div class="body">
			{#each columns as column, i (i)}
				<div class="column" class:moods={column.level === 'mood'} data-testid="library-level-{column.level}">
					<h3 class="label">{LEVEL_WORDS[column.level]()}</h3>
					<div class="rows">
						{#each column.rows as l (levelKey(l))}
							<ListRow dense label={l.name} title={l.mood ? `${l.address} · ${l.mood}` : l.address} selected={shelf.chosen[i] === levelKey(l)} onclick={() => shelf.choose(i, levelKey(l))} />
						{/each}
					</div>
				</div>
			{/each}
			<div class="beside">
				{#if beside.length}
					<h3 class="label">{m['library.files']()}</h3>
					<div class="files">
						<LibraryGrid entries={all ? beside : beside.slice(0, FEW)} {tile} label={m['library.files']()} onopen={open} {onmenu} />
						{#if !all && beside.length > FEW}<Button variant="quiet" onclick={() => (all = true)} data-testid="library-all-files">{m['library.all_files']({ count: beside.length })}</Button>{/if}
					</div>
				{/if}
				{#if at.usedIn.length}
					<h3 class="label">{m['library.used_in']()}</h3>
					<ul class="used" data-testid="library-used-in">
						{#each at.usedIn as link (link.address + link.as)}
							<li>
								<button type="button" class="link" data-control-exempt="text link" title={`${link.address} · ${m[`library.as_${link.as}`]()}`} onclick={() => onopen(link.address)}>{linkLabel(link)}</button>
								<span class="as">{m[`library.as_${link.as}`]()}</span>
								<Icon name="chevron-right" size="0.8rem" />
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		</div>
	{:else}
		<p class="sc-empty">{m['library.loading']()}</p>
	{/if}
</div>

<style>
	.step {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		min-width: 0;
	}
	.crumb {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.35rem;
		min-width: 0;
	}
	.part {
		max-width: 14rem;
		overflow: hidden;
		padding: 0;
		border: 0;
		background: none;
		color: var(--tool-muted);
		font: inherit;
		text-overflow: ellipsis;
		white-space: nowrap;
		cursor: pointer;
	}
	.part:hover,
	.part.here {
		color: var(--tool-text);
	}
	.part.here {
		font-size: var(--editor-title, 1rem);
	}
	.sep {
		color: var(--tool-muted);
	}
	.avatar {
		width: 1.6rem;
		height: 1.6rem;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: 50%;
		object-fit: cover;
	}
	.sp {
		flex: 1 1 auto;
	}
	.body {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		gap: 1rem;
	}
	.column {
		flex: 0 1 9rem;
		min-width: 7rem;
	}
	.column.moods {
		flex-basis: 16rem;
	}
	.column.moods .rows {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(7rem, 1fr));
		column-gap: 0.3rem;
	}
	.label {
		margin: 0 0 0.35rem;
		color: var(--tool-muted);
		font-size: var(--editor-label);
		font-weight: 400;
		letter-spacing: var(--editor-label-spacing, 0.08em);
		text-transform: uppercase;
	}
	.beside {
		flex: 1 1 18rem;
		min-width: 14rem;
		padding-left: 1rem;
		border-left: var(--editor-edge) solid var(--tool-line);
	}
	.files {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.3rem;
		margin-bottom: 1rem;
	}
	.files > :global(*) {
		align-self: stretch;
	}
	.files > :global(.kit-button) {
		align-self: flex-start;
	}
	.used {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.used li {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-height: var(--editor-tree-row);
		color: var(--tool-muted);
	}
	.link {
		flex: 0 1 auto;
		min-width: 0;
		overflow: hidden;
		padding: 0;
		border: 0;
		background: none;
		color: var(--tool-text);
		font: inherit;
		text-decoration: underline;
		text-underline-offset: 0.2em;
		text-overflow: ellipsis;
		white-space: nowrap;
		cursor: pointer;
	}
	.as {
		flex: 1 1 auto;
		font-size: var(--editor-label);
	}
</style>
