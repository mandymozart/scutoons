<script lang="ts">
	import { untrack } from 'svelte';
	import { Icon, SortGrip, sortable } from '@scutoons/ui';
	import { getAt, previewText, type CharacterRegistry, type CollagePiece, type MoodVocabulary, type Path, type SliceLayerKind } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { asset, toonUrl } from '$lib/utils/cdn.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { ChapterView } from '$lib/editor/chapter-state.svelte.ts';
	import { triggerWord } from '$lib/editor/vn-forms.ts';
	import { itemCatalog, itemName } from '$lib/editor/catalog.svelte.ts';
	import { followPieces } from '$lib/editor/collage-edit.ts';
	import { groupLayerRows, moveGroupLayerRow } from '$lib/editor/group-layers.ts';
	import type { DrawnBox } from '$lib/editor/strip-anchor.ts';
	import type { CanvasSelection } from '$lib/components/slice/slice-canvas.svelte';
	import { hasShapePiece, pieceLabel } from '$lib/editor/collage-words.ts';
	import { shapeLibrary } from '$lib/editor/shapes.svelte.ts';
	import { toast } from '$lib/toasts.ts';

	/**
	 * The group's ONE layer list (BACKLOG 10.43, 10.54; research/COLLAGE.md §3 "Layers list"; owner 2026-10-20 – flat, no
	 * slices, research/TEXT-ACROSS-SLICES.md §4.1 E2), front first: the slices' stacks – text boxes, placed items and the
	 * click / hover areas of events and samples (edited in their own tabs), a later slice's in front of an earlier one's –
	 * then, under a rule, the collage pieces (art always lies under the stacks). A drag reorders (group-layers.ts – one
	 * edit, one undo step); a text box or item dropped among another slice's rows is re-anchored there (DECISIONS
	 * 2026-10-20); pieces and areas stay with their slice, the stack never goes among the pieces. A row of another slice
	 * opens that slice (`onpick`).
	 */
	let {
		view,
		doc,
		groupPath,
		open,
		toon,
		locale,
		registry = null,
		moods = null,
		boxOf,
		onpick,
		onlongpress
	}: {
		view: ChapterView;
		doc: ContentDoc;
		groupPath: Path;
		/** the open slice (the selection's) */
		open: number;
		/** a slice's drawn box on the strip (re-anchoring keeps the place on screen) */
		boxOf: (slice: number) => DrawnBox | null;
		/** a row was picked (its slice opens, the row's entry is selected) */
		onpick: (slice: number, selection: CanvasSelection) => void;
		/** a finger held on a row (500 ms): the phone's context – copy · cut · delete · edit (E3) */
		onlongpress?: (slice: number, selection: CanvasSelection, box: DOMRect) => void;
		toon: string;
		/** the language the canvas shows */
		locale: string;
		/** a character picture's name (else its file's) */
		registry?: CharacterRegistry | null;
		moods?: MoodVocabulary | null;
	} = $props();

	interface LayerRow {
		/** the slice it is anchored on */
		slice: number;
		kind: SliceLayerKind;
		index: number;
		/** the item's name / the start of the text (else the box's id) / the piece's character or file */
		name: string;
		/** tooltip: a text box's id */
		title?: string;
		/** the item's icon / the piece's file */
		icon?: string;
		/** the glyph without an icon (Feather / icons-extra): a text box `type`, an event area `zap`, a sample area `music` (as their tabs), a piece `shapes` (the piece tool's) */
		glyph: string;
		pickup: boolean;
		/** an event area: all of its events (they move together) */
		events?: number[];
	}

	const slices = $derived<any[]>(doc.get(doc.primary, [...groupPath, 'slices']) ?? []);
	/** the rows front first (the top row is drawn on top); the pieces last */
	const rows = $derived.by<LayerRow[]>(() =>
		groupLayerRows(slices).map(({ slice: at, ref: { kind, index, events } }): LayerRow => ({ slice: at, ...rowOf(slices[at], kind, index, events) }))
	);
	function rowOf(slice: any, kind: SliceLayerKind, index: number, events?: number[]): Omit<LayerRow, 'slice'> {
		{
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
				return { kind, index, name: pieceLabel(piece, registry, moods), icon: src || undefined, glyph: 'shapes', pickup: false };
			}
			const it = slice?.items?.[index];
			const def = itemCatalog.catalog?.items.find((x) => x.id === it?.item);
			return { kind, index, name: itemName(def, it?.item ?? '?'), icon: def?.icon ? asset(def.icon) : undefined, glyph: 'box', pickup: it?.action === 'pickup' };
		}
	}
	/** a shape's name is its library title (`pieceLabel` only reads it): the list loads the library once it holds a shape */
	const listsShape = $derived(slices.some((s) => hasShapePiece(s?.pieces)));
	$effect(() => {
		if (listsShape) untrack(() => void shapeLibrary.load());
	});
	/** the first piece row: the rule above it marks where the art starts */
	const firstPiece = $derived(rows.findIndex((r) => r.kind === 'piece'));
	/** the row of the canvas selection (an event area: any of its events) */
	const isSelected = (row: LayerRow) => row.slice === open && view.selection?.kind === row.kind && (row.events ?? [row.index]).includes(view.selection.index);
	/**
	 * a drag in the list (front first) – one edit, one undo step; refused (pieces / areas across): a hint, nothing changes.
	 * The moved row stays / becomes the selection where it is now (its slice opens); a pieces' reorder of the open slice
	 * renumbers `pieces[]`: the selected piece goes with its piece
	 */
	function moveLayer(from: number, to: number) {
		const row = rows[from];
		if (!row) return;
		const wasSelected = isSelected(row);
		let done = { ok: false, why: 'none' } as ReturnType<typeof moveGroupLayerRow>;
		const before = groupLayerRows(slices);
		const target = before[to];
		// a refused move changes nothing: no edit (no undo step)
		if (!target || (row.kind === 'piece') !== (target.ref.kind === 'piece') || (row.slice !== target.slice && row.kind !== 'dialog' && row.kind !== 'item')) {
			if (from !== to) toast(row.kind === 'piece' || target?.ref.kind === 'piece' ? m['creator.layers_pieces_under']() : m['creator.layers_area_stays'](), { type: 'info' });
			return;
		}
		doc.edit((file) => {
			const list = getAt(file, [...groupPath, 'slices']);
			if (Array.isArray(list)) done = moveGroupLayerRow(list, from, to, boxOf);
		});
		if (!done.ok) return;
		if (done.slice !== open || (wasSelected && done.kind !== 'piece')) onpick(done.slice, { kind: done.kind, index: done.index });
		else if (done.moved) view.selection = followPieces(view.selection, done.moved);
	}
	/** the long press on a row's button (a finger, not moved): its timer; the tap that ends it does nothing else */
	let hold: { timer: ReturnType<typeof setTimeout>; x: number; y: number; fired: boolean } | null = null;
	function holdStart(event: PointerEvent, row: LayerRow) {
		if (!onlongpress || event.pointerType !== 'touch') return;
		const el = event.currentTarget as HTMLElement;
		hold = { x: event.clientX, y: event.clientY, fired: false, timer: setTimeout(() => {
			if (!hold) return;
			hold.fired = true;
			onlongpress?.(row.slice, { kind: row.kind, index: row.index }, el.getBoundingClientRect());
		}, 500) };
	}
	function holdMove(event: PointerEvent) {
		if (hold && !hold.fired && Math.hypot(event.clientX - hold.x, event.clientY - hold.y) > 8) holdEnd();
	}
	function holdEnd() {
		if (hold && !hold.fired) clearTimeout(hold.timer);
		if (hold && !hold.fired) hold = null;
	}
	function rowClick(row: LayerRow) {
		if (hold?.fired) {
			hold = null;
			return;
		}
		hold = null;
		onpick(row.slice, { kind: row.kind, index: row.index });
	}
	/** the row's testids: the open slice's as before (`dialog-0` …), another slice's with its number (`s3-dialog-0`) */
	const rowTestid = (row: LayerRow) =>
		(row.slice === open ? '' : `s${row.slice}-`) +
		(row.kind === 'dialog' ? `dialog-${row.index}` : row.kind === 'item' ? `slice-item-${row.index}` : row.kind === 'piece' ? `slice-piece-${row.index}` : `layer-${row.kind}-row-${row.index}`);
</script>

{#if rows.length}
	<ul class="layers" use:sortable={{ onmove: moveLayer }} data-testid="layers">
		{#each rows as row, i (`${row.slice}:${row.kind}:${row.index}`)}
			<!-- the canvas selection: the canvas + this inspector are ONE region ('slice', the chapter editor) -->
			<li data-sort-row class:sc-selected={isSelected(row)} class:boundary={i === firstPiece && i > 0} data-slice={row.slice} data-testid="layer-{row.slice === open ? '' : `s${row.slice}-`}{row.kind}-{row.index}">
				<SortGrip label={m['creator.sort_move']({ name: row.name })} title={m['creator.sort_hint']()} />
				<button type="button" class="ghost row" title={row.title} onclick={() => rowClick(row)} onpointerdown={(e) => holdStart(e, row)} onpointermove={holdMove} onpointerup={holdEnd} onpointercancel={holdEnd} oncontextmenu={(e) => onlongpress && e.preventDefault()} data-testid={rowTestid(row)}>
					<span class="thumb" class:sc-checker={row.kind === 'piece'}>
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
		background-color: var(--paper);
		color: var(--ink);
		/* a piece's file on the checker (tools.css .sc-checker – white shapes stay visible), small for the small box */
		--checker-size: 0.5rem;
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
