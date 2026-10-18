<script lang="ts">
	import { untrack } from 'svelte';
	import { Icon, SortGrip, sortable } from '@scutoons/ui';
	import type { CharacterRegistry, CollagePiece, MoodVocabulary } from '@scutoons/content';
	import { collageRows, type CollageRow } from '$lib/editor/score-collage.ts';
	import { hasShapePiece, pieceLabel } from '$lib/editor/collage-words.ts';
	import { shapeLibrary } from '$lib/editor/shapes.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A score's collage pieces (research/COLLAGE.md §4.4), front first: the pieces over the notes, then – under a rule –
	 * the pieces behind them (`collageRows`). A drag reorders within its side (the side is the inspector's Over / Behind);
	 * a click selects the piece and the stage turns to its page. ⚠ = its bar is not in the file (readers skip it).
	 */
	let {
		pieces,
		selected,
		missing,
		registry = null,
		moods = null,
		onselect,
		onmove
	}: {
		pieces: CollagePiece[];
		selected: string | null;
		/** the ids of the pieces whose bar the file lacks */
		missing: ReadonlySet<string>;
		registry?: CharacterRegistry | null;
		moods?: MoodVocabulary | null;
		onselect: (id: string) => void;
		/** a drag in the list (`collageRows` order) */
		onmove: (from: number, to: number) => void;
	} = $props();

	const rows = $derived(collageRows(pieces));
	/** a shape's name is its library title (`pieceLabel` only reads it): the list loads the library once it holds a shape */
	const listsShape = $derived(hasShapePiece(pieces));
	$effect(() => {
		if (listsShape) untrack(() => void shapeLibrary.load());
	});
	/** the first row behind the notes: the rule above it */
	const firstBehind = $derived(rows.findIndex((r) => r.behind));
	const pieceOf = (row: CollageRow) => pieces[row.index];
	/** where it sits: its bar (+ staff), or the page that holds the bar */
	function placeOf(piece: CollagePiece): string {
		const at = piece.at;
		if (!at) return '';
		const bar = m['creator.collage_row_bar']({ bar: at.nth && at.nth > 1 ? `${at.bar} (${at.nth})` : at.bar });
		return at.page ? `${bar} · ${m['creator.collage_row_page']()}` : at.staff && at.staff > 1 ? `${bar} · ${m['creator.collage_row_staff']({ staff: at.staff })}` : bar;
	}
</script>

{#if rows.length}
	<ul class="pieces" use:sortable={{ onmove }} data-testid="collage-list">
		<!-- keyed by the row's place, not the piece's id: two pieces of one id (or '') still list – to select and remove -->
		{#each rows as row, i (row.index)}
			{@const piece = pieceOf(row)}
			{@const name = pieceLabel(piece, registry, moods)}
			<li data-sort-row class:sc-selected={piece?.id === selected} class:boundary={i === firstBehind && i > 0} data-testid="collage-row-{piece?.id}">
				<SortGrip label={m['creator.sort_move']({ name })} title={m['creator.sort_hint']()} />
				<button type="button" class="ghost row" title={row.behind ? m['creator.piece_behind_hint']() : m['creator.piece_over_hint']()} onclick={() => piece && onselect(piece.id)} data-testid="collage-select-{piece?.id}">
					<span class="thumb sc-checker">{#if piece?.src}<img src={asset(piece.src)} alt="" draggable="false" />{:else}<Icon name="shapes" size="1rem" />{/if}</span>
					<span class="name">{name}</span>
					<span class="where">{piece ? placeOf(piece) : ''}</span>
				</button>
				{#if piece && missing.has(piece.id)}
					<span class="missing" title={m['creator.collage_missing_bar']({ bar: piece.at?.bar ?? '' })} data-testid="collage-missing-{piece.id}"><Icon name="alert-triangle" size="1rem" label={m['creator.collage_missing_bar']({ bar: piece.at?.bar ?? '' })} /></span>
				{/if}
			</li>
		{/each}
	</ul>
{/if}

<style>
	/* one line per piece – grip, file, name, its bar (as the slice's Layers list) */
	.pieces {
		list-style: none;
		margin: 0;
		padding: 0;
		border: var(--border-width-regular) solid var(--ink);
	}
	.pieces li {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding-inline: var(--space-05);
	}
	.pieces li + li {
		border-top: var(--border-width-hairline) solid var(--ink);
	}
	/* where the pieces behind the notes start */
	.pieces li.boundary {
		border-top-width: var(--border-width-regular);
	}
	.row {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		justify-content: flex-start;
		gap: var(--space-05);
		height: calc(var(--control-height) - var(--border-width-hairline));
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		text-align: left;
	}
	/* the file on the checker (tools.css .sc-checker – white shapes stay visible), small for the small box */
	.thumb {
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1.75rem;
		height: 1.75rem;
		box-sizing: border-box;
		border: var(--border-width-hairline) solid var(--ink);
		--checker-size: 0.5rem;
		color: var(--ink);
	}
	.thumb img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
	.name {
		min-width: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.where {
		margin-left: auto;
		flex: none;
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}
	.missing {
		flex: none;
		display: inline-flex;
		color: var(--state-warning);
	}
</style>
