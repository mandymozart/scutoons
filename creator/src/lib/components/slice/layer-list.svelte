<script lang="ts">
	import { Icon, SortGrip, sortable } from '@scutoons/ui';
	import { getAt, moodLabel, pieceName, previewText, type CharacterRegistry, type CollagePiece, type MoodVocabulary, type Path, type SliceLayerKind } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { asset, toonUrl } from '$lib/utils/cdn.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { ChapterView } from '$lib/editor/chapter-state.svelte.ts';
	import { triggerWord } from '$lib/editor/vn-forms.ts';
	import { itemCatalog, itemName } from '$lib/editor/catalog.svelte.ts';
	import { canMoveLayerRow, followPieces, layerRows, moveLayerRows } from '$lib/editor/collage-edit.ts';
	import { shownCharacterName, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { toast } from '$lib/toasts.ts';

	/**
	 * The slice's layer list (BACKLOG 10.43, 10.54; research/COLLAGE.md §3 "Layers list"), front first: the stack – text
	 * boxes, placed items and the click / hover areas of events and samples (edited in their own tabs) – then, under a
	 * rule, the collage pieces (the front piece first): art always lies under the stack. A drag reorders within its side
	 * (`moveLayerRows`: pieces among pieces, the stack among the stack – one edit, one undo step); across is refused.
	 */
	let {
		view,
		doc,
		slice,
		slicePath,
		toon,
		locale,
		registry = null,
		moods = null
	}: {
		view: ChapterView;
		doc: ContentDoc;
		slice: any;
		slicePath: Path;
		toon: string;
		/** the language the canvas shows */
		locale: string;
		/** a character picture's name (else its file's) */
		registry?: CharacterRegistry | null;
		moods?: MoodVocabulary | null;
	} = $props();

	interface LayerRow {
		kind: SliceLayerKind;
		index: number;
		/** the item's name / the start of the text (else the box's id) / the piece's character or file */
		name: string;
		/** tooltip: a text box's id */
		title?: string;
		/** the item's icon / the piece's file */
		icon?: string;
		/** the Feather glyph without an icon: a text box `type`, an event area `zap`, a sample area `music` (as their tabs), a piece `scissors` */
		glyph: string;
		pickup: boolean;
		/** an event area: all of its events (they move together) */
		events?: number[];
	}

	/** a piece's name: a character picture by its name · mood, else its file */
	function pieceRowName(piece: CollagePiece | undefined): string {
		if (!piece) return '?';
		const figure = piece.figure;
		const character = figure ? registry?.characters.find((c) => c.id === figure.id) : undefined;
		if (!figure || !character) return pieceName(piece);
		return [shownCharacterName(character, figure.persona), figure.mood && moodLabel(figure.mood, textLanguage.names, moods)].filter(Boolean).join(' · ');
	}

	/** the rows front first (the top row is drawn on top); the pieces last */
	const rows = $derived.by<LayerRow[]>(() =>
		layerRows(slice).map(({ kind, index, events }): LayerRow => {
			if (kind === 'dialog') {
				const d = slice?.dialog?.[index];
				const text = previewText(d?.text, [locale, doc.textSource]).trim().slice(0, 40);
				return { kind, index, name: text || d?.id || `#${index + 1}`, title: d?.id, glyph: 'type', pickup: false };
			}
			if (kind === 'event') {
				// an area: its events' types (the first, + how many more) and its trigger, as the Events tab writes them
				const list = (events ?? [index]).map((i) => slice.events?.[i]);
				const more = list.length > 1 ? ` +${list.length - 1}` : '';
				const name = `${list[0]?.type ?? '?'}${more} · ${triggerWord(list[0])}`;
				return { kind, index, name, title: list.map((e) => e?.type).join(', '), glyph: 'zap', pickup: false, events };
			}
			if (kind === 'sample') {
				const sample = slice.audio?.audio?.samples?.[index];
				return { kind, index, name: sample?.id ? `${sample.id} · ${triggerWord(sample)}` : triggerWord(sample), glyph: 'music', pickup: false };
			}
			if (kind === 'piece') {
				const piece = slice.pieces?.[index] as CollagePiece | undefined;
				const src = typeof piece?.src === 'string' && piece.src ? toonUrl(toon, piece.src) : '';
				return { kind, index, name: pieceRowName(piece), icon: src || undefined, glyph: 'scissors', pickup: false };
			}
			const it = slice?.items?.[index];
			const def = itemCatalog.catalog?.items.find((x) => x.id === it?.item);
			return { kind, index, name: itemName(def, it?.item ?? '?'), icon: def?.icon ? asset(def.icon) : undefined, glyph: 'box', pickup: it?.action === 'pickup' };
		})
	);
	/** the first piece row: the rule above it marks where the art starts */
	const firstPiece = $derived(rows.findIndex((r) => r.kind === 'piece'));
	/** the row of the canvas selection (an event area: any of its events) */
	const isSelected = (row: LayerRow) => view.selection?.kind === row.kind && (row.events ?? [row.index]).includes(view.selection.index);
	/**
	 * a drag in the list (front first) within its side – one edit, one undo step; across = a hint, nothing changes. The
	 * pieces' reorder renumbers `pieces[]`: the selected piece goes with its piece (the stack keeps its indices)
	 */
	function moveLayer(from: number, to: number) {
		if (!canMoveLayerRow(layerRows(slice), from, to)) {
			if (from !== to) toast(m['creator.layers_pieces_under'](), { type: 'info' });
			return;
		}
		doc.edit((file) => {
			const moved = moveLayerRows(getAt(file, slicePath), from, to);
			if (moved) view.selection = followPieces(view.selection, moved);
		});
	}
	/** the row's button testid (the text box / item ones as before) */
	const rowTestid = (row: LayerRow) =>
		row.kind === 'dialog' ? `dialog-${row.index}` : row.kind === 'item' ? `slice-item-${row.index}` : row.kind === 'piece' ? `slice-piece-${row.index}` : `layer-${row.kind}-row-${row.index}`;
</script>

{#if rows.length}
	<ul class="layers" use:sortable={{ onmove: moveLayer }} data-testid="layers">
		{#each rows as row, i (`${row.kind}:${row.index}`)}
			<!-- the canvas selection: the canvas + this inspector are ONE region ('slice', the chapter editor) -->
			<li data-sort-row class:sc-selected={isSelected(row)} class:boundary={i === firstPiece && i > 0} data-testid="layer-{row.kind}-{row.index}">
				<SortGrip label={m['creator.sort_move']({ name: row.name })} title={m['creator.sort_hint']()} />
				<button type="button" class="ghost row" title={row.title} onclick={() => view.select({ kind: row.kind, index: row.index })} data-testid={rowTestid(row)}>
					<span class="thumb" class:checker={row.kind === 'piece'}>
						{#if row.icon}<img src={row.icon} alt="" draggable="false" />{:else}<Icon name={row.glyph} size="1rem" />{/if}
					</span>
					<span class="name">{row.name}</span>
				</button>
				{#if row.pickup}
					<span class="pickup" title={m['creator.slice_item_action_pickup']()}><Icon name="download" size="1rem" label={m['creator.slice_item_action_pickup']()} /></span>
				{/if}
			</li>
		{/each}
	</ul>
{/if}

<style>
	/* the layer stack: one line per entry – grip, icon, name, the pickup mark */
	.layers {
		list-style: none;
		margin: 0;
		padding: 0;
		border: var(--border-width-regular) solid var(--ink);
	}
	.layers li {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding-inline: var(--space-05);
	}
	.layers li + li {
		border-top: var(--border-width-hairline) solid var(--ink);
	}
	/* where the collage pieces start (they always lie under the stack) */
	.layers li.boundary {
		border-top-width: var(--border-width-regular);
	}
	.layers .row {
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
	.thumb {
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1.75rem;
		height: 1.75rem;
		box-sizing: border-box;
		border: var(--border-width-hairline) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
	}
	/* a piece's file on a checker (white shapes stay visible) */
	.thumb.checker {
		background-image: conic-gradient(var(--surface-hover) 25%, transparent 0 50%, var(--surface-hover) 0 75%, transparent 0);
		background-size: 0.5rem 0.5rem;
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
	.pickup {
		flex: none;
		display: inline-flex;
	}
</style>
