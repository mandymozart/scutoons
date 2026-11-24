<script lang="ts" module>
	/** the models whose still this visit already sent up */
	const sent = new Set<string>();
</script>

<script lang="ts">
	import { assetThumbKey, THUMB_HEIGHT, THUMB_WIDTH, type LibraryEntry } from '@scutoons/api';
	import { Icon } from '@scutoons/ui';
	import { Thumb } from '@scutoons/ui/editor';
	import { ModelThumb } from '@scutoons/media';
	import { library } from '$lib/editor/library.svelte.ts';
	import { filePath } from '$lib/editor/library-list.ts';
	import { thumbs, urlToStill } from '$lib/shell/thumbs.svelte.ts';
	import { iconOf } from '$lib/shell/address.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * One entry of the Library's answer as a tile (or a list row): its picture in the kit's Thumb (a video's frame, a
	 * character's look …) or its type's / kind's icon with "No still yet" as hover text, its name and what it is. A 3D
	 * model without a still renders one here and stores it once for everyone (`PUT /thumbs`, #842). A click selects
	 * (Ctrl / ⌘ adds, Shift a run), a double click or Enter steps into a thing or uses a file, right-click or ⋯ opens a
	 * file's menu, a file drags onto a stage or a slot.
	 */
	let {
		entry,
		selected = false,
		row = false,
		fresh = false,
		onpress,
		onopen,
		onmenu
	}: {
		entry: LibraryEntry;
		selected?: boolean;
		row?: boolean;
		fresh?: boolean;
		onpress: (keys: { shift: boolean; toggle: boolean }) => void;
		onopen: () => void;
		onmenu?: (x: number, y: number) => void;
	} = $props();

	const KIND_ICONS: Record<string, string> = { image: 'image', video: 'film', audio: 'music', model: 'box', animation: 'activity', file: 'file' };
	const TYPE_ICONS: Record<string, string> = { place: 'map-pin', show: 'play', instrument: 'music', person: 'user' };
	const path = $derived(filePath(entry));
	const icon = $derived(entry.kind ? (KIND_ICONS[entry.kind] ?? 'file') : (TYPE_ICONS[entry.type ?? ''] ?? iconOf(entry.address)));
	const what = $derived(entry.type ? m[`library.type_${entry.type}`]() : m[`library.file_${entry.kind ?? 'file'}`]());
	const family = $derived(path ? library.byAddress.get(entry.address) : undefined);
	/** a model whose still is not stored yet: rendered here, sent up once */
	const renders = $derived(entry.kind === 'model' && !entry.picture && !!path);

	async function store(png: string) {
		const id = family?.versions.find((v) => v.path === family.main)?.asset ?? family?.assets[0];
		if (!id || sent.has(id)) return;
		sent.add(id);
		const still = await urlToStill(png);
		if (still) await thumbs.put(assetThumbKey(id), still);
	}

	function drag(event: DragEvent) {
		if (!event.dataTransfer || !path) return event.preventDefault();
		event.dataTransfer.effectAllowed = 'copy';
		event.dataTransfer.setData('application/x-scutoons-asset', JSON.stringify({ path, key: family?.key ?? entry.address, kind: entry.kind }));
		event.dataTransfer.setData('text/plain', path);
		library.dragging = path;
	}

	function key(event: KeyboardEvent) {
		if (event.key === 'Enter') onopen();
		else if (event.key === ' ') (event.preventDefault(), onpress({ shift: event.shiftKey, toggle: event.ctrlKey || event.metaKey }));
		else if (onmenu && (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10'))) {
			event.preventDefault();
			const r = (event.currentTarget as HTMLElement).getBoundingClientRect();
			onmenu(r.left, r.bottom);
		}
	}

	function menu(event: MouseEvent) {
		if (!onmenu) return;
		event.preventDefault();
		event.stopPropagation();
		if (!selected) onpress({ shift: false, toggle: false });
		onmenu(event.clientX, event.clientY);
	}
</script>

<div
	class="tile"
	class:row
	class:selected
	role="option"
	tabindex="0"
	aria-selected={selected}
	title={entry.name}
	draggable={!!path}
	data-control-exempt="library tile"
	data-testid="library-tile"
	data-address={entry.address}
	onclick={(e) => onpress({ shift: e.shiftKey, toggle: e.ctrlKey || e.metaKey })}
	ondblclick={onopen}
	oncontextmenu={menu}
	onkeydown={key}
	ondragstart={drag}
	ondragend={() => (library.dragging = null)}
>
	{#if renders && path}
		<Thumb {icon} empty={m['library.no_still']()} class="pic">
			<ModelThumb {path} size={row ? '0.9rem' : '1.4rem'} still={{ width: THUMB_WIDTH, height: THUMB_HEIGHT }} onstill={(png) => void store(png)} />
		</Thumb>
	{:else}
		<Thumb src={entry.picture} {icon} size={row ? '0.9rem' : '1.4rem'} empty={m['library.no_still']()} class="pic" />
	{/if}
	<span class="text">
		<span class="name">{entry.name}</span>
		<span class="what" class:fresh>{fresh ? m['library.just_uploaded']() : what}</span>
	</span>
	{#if onmenu && path}
		<button
			type="button"
			class="more"
			data-control-exempt="tile action"
			title={m['library.actions']()}
			aria-label={m['library.actions']()}
			onclick={(e) => {
				e.stopPropagation();
				const r = e.currentTarget.getBoundingClientRect();
				if (!selected) onpress({ shift: false, toggle: false });
				onmenu(r.left, r.bottom);
			}}><Icon name="more-horizontal" size="0.85rem" /></button
		>
	{/if}
</div>

<style>
	.tile {
		position: relative;
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 0;
		cursor: pointer;
		outline: none;
		font-size: var(--editor-small);
	}
	.tile :global(.pic) {
		border-radius: var(--kit-radius-tile);
	}
	.tile:focus-visible :global(.pic),
	.tile.selected :global(.pic) {
		outline: 0.125rem solid var(--tool-text);
		outline-offset: 0.0625rem;
	}
	.text {
		display: flex;
		flex-direction: column;
		min-width: 0;
		padding: 0.1rem 0.1rem 0;
	}
	.name,
	.what {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.name {
		color: var(--tool-text);
	}
	.what {
		color: var(--tool-muted);
		font-size: var(--editor-label);
	}
	.what.fresh {
		color: var(--tool-text);
	}
	.more {
		position: absolute;
		top: 0.3rem;
		right: 0.3rem;
		width: 1.5rem;
		height: 1.5rem;
		display: none;
		place-items: center;
		padding: 0;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: 50%;
		background: var(--tool-popover);
		color: var(--tool-text);
		text-shadow: var(--tool-halo);
		cursor: pointer;
	}
	.tile:hover .more,
	.tile:focus-within .more {
		display: inline-grid;
	}

	/* list rows */
	.tile.row {
		flex-direction: row;
		align-items: center;
		gap: 0.5rem;
		min-height: var(--editor-tree-row);
		padding: 0 0.4rem 0 0.2rem;
		border-radius: var(--editor-radius);
	}
	.tile.row:hover {
		background: var(--tool-hover);
	}
	.tile.row.selected {
		outline: 0.125rem solid var(--tool-text);
	}
	.row :global(.pic) {
		flex: none;
		width: 2.4rem;
	}
	.row.selected :global(.pic) {
		outline: none;
	}
	.row .text {
		flex: 1 1 auto;
		flex-direction: row;
		gap: 0.5rem;
		align-items: baseline;
		padding: 0;
	}
	.row .name {
		flex: 1 1 auto;
	}
	.row .more {
		position: static;
	}
</style>
