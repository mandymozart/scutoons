<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { ModelThumb, bytes, formatDuration, previewPicture, previewStill, type AssetFamily } from '@scutoons/media';
	import { asset } from '$lib/utils/cdn.ts';
	import { library } from '$lib/editor/library.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * One asset family as a card (or a list row): the preview the core chose (a picture, a model's still, the kind),
	 * its versions as sheets behind it (a stack), the name and one meta line – pixels · file size · version · further
	 * uses. A click selects (Ctrl / ⌘ adds, Shift a run), a double click uses, right-click or ⋯ opens the menu, a drag
	 * hands the file on (`application/x-scutoons-asset`: a stage, a tree row).
	 */
	let {
		family,
		selected = false,
		row = false,
		fresh = false,
		stacked = true,
		onpress,
		onuse,
		onmenu
	}: {
		family: AssetFamily;
		selected?: boolean;
		row?: boolean;
		/** just uploaded */
		fresh?: boolean;
		/** the levels as a stack (the browser's toggle: single shows every file on its own) */
		stacked?: boolean;
		onpress: (keys: { shift: boolean; toggle: boolean }) => void;
		onuse: () => void;
		onmenu: (x: number, y: number) => void;
	} = $props();

	const ICONS: Record<AssetFamily['kind'], string> = { image: 'image', video: 'film', audio: 'music', model: 'box', animation: 'activity', file: 'file' };
	const picture = $derived(previewPicture(family.preview));
	const still = $derived(previewStill(family.preview));
	const meta = $derived(
		[
			fresh ? m['library.just_uploaded']() : '',
			family.width && family.height ? `${family.width}×${family.height}` : formatDuration(family.duration),
			bytes(family.size),
			family.version > 1 ? `v${family.version}` : '',
			family.usages > 1 ? m['library.more_uses']({ count: family.usages - 1 }) : ''
		]
			.filter(Boolean)
			.join(' · ')
	);

	function drag(event: DragEvent) {
		if (!event.dataTransfer) return;
		event.dataTransfer.effectAllowed = 'copy';
		event.dataTransfer.setData('application/x-scutoons-asset', JSON.stringify({ path: family.main, key: family.key, kind: family.kind }));
		event.dataTransfer.setData('text/plain', family.main);
		library.dragging = family.main;
	}

	function key(event: KeyboardEvent) {
		if (event.key === 'Enter') onuse();
		else if (event.key === ' ') (event.preventDefault(), onpress({ shift: event.shiftKey, toggle: event.ctrlKey || event.metaKey }));
		else if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
			event.preventDefault();
			const r = (event.currentTarget as HTMLElement).getBoundingClientRect();
			onmenu(r.left, r.bottom);
		}
	}

	function menu(event: MouseEvent) {
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
	class:stack={stacked && !row && family.versions.length > 1}
	role="option"
	tabindex="0"
	aria-selected={selected}
	title={family.key}
	draggable="true"
	data-control-exempt="asset tile"
	data-testid="library-tile"
	data-key={family.key}
	onclick={(e) => onpress({ shift: e.shiftKey, toggle: e.ctrlKey || e.metaKey })}
	ondblclick={onuse}
	oncontextmenu={menu}
	onkeydown={key}
	ondragstart={drag}
	ondragend={() => (library.dragging = null)}
>
	<span class="preview">
		{#if picture}
			<img src={asset(picture)} alt="" loading="lazy" draggable="false" />
		{:else if still}
			<ModelThumb path={family.main} size={row ? '1rem' : '1.6rem'} />
		{:else}
			<span class="glyph"><Icon name={ICONS[family.kind]} size={row ? '1rem' : '1.6rem'} /></span>
		{/if}
		{#if !row && family.versions.length > 1 && stacked}<span class="count mono" title={m['library.versions_count']({ count: family.versions.length })}>{family.versions.length}</span>{/if}
	</span>
	<span class="text">
		<span class="name">{family.title}</span>
		<span class="meta mono" class:fresh>{meta}</span>
	</span>
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
</div>

<style>
	.tile {
		position: relative;
		display: flex;
		flex-direction: column;
		min-width: 0;
		cursor: pointer;
		outline: none;
		font-size: var(--editor-small);
	}
	.preview {
		position: relative;
		aspect-ratio: 1;
		display: grid;
		place-items: center;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: var(--editor-radius);
		background: var(--tool-ground);
		overflow: hidden;
	}
	.tile:focus-visible .preview,
	.tile.selected .preview {
		outline: 0.125rem solid var(--tool-text);
		outline-offset: 0.0625rem;
	}
	/* a version stack (OSCT #331): two sheets behind the tile; every card keeps the pile's room so they line up */
	.tile:not(.row) {
		padding: 0.4rem 0.4rem 0 0;
	}
	.tile.stack .preview {
		overflow: visible;
		box-shadow:
			0.2rem -0.2rem 0 -0.0625rem var(--tool-head),
			0.2rem -0.2rem 0 0 var(--tool-line),
			0.4rem -0.4rem 0 -0.0625rem var(--tool-head),
			0.4rem -0.4rem 0 0 var(--tool-line);
	}
	.preview img {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: contain;
		border-radius: inherit;
	}
	.glyph {
		color: var(--tool-muted);
	}
	.count {
		position: absolute;
		right: 0.25rem;
		bottom: 0.25rem;
		padding: 0 0.3rem;
		border-radius: 0.3rem;
		background: var(--tool-head);
		color: var(--tool-muted);
		font-size: var(--editor-label);
	}
	.text {
		display: flex;
		flex-direction: column;
		min-width: 0;
		padding: 0.2rem 0.1rem 0;
	}
	.name,
	.meta {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.name {
		color: var(--tool-text);
	}
	.meta {
		color: var(--tool-muted);
		font-size: var(--editor-label);
	}
	.meta.fresh {
		color: var(--tool-text);
	}
	.more {
		position: absolute;
		top: 0.6rem;
		right: 0.6rem;
		width: 1.5rem;
		height: 1.5rem;
		display: none;
		place-items: center;
		padding: 0;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: 50%;
		background: var(--tool-head);
		color: var(--tool-text);
		cursor: pointer;
	}
	.tile:hover .more,
	.tile:focus-within .more {
		display: inline-grid;
	}
	.mono {
		font-family: var(--editor-mono, ui-monospace, monospace);
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
	.row .preview {
		flex: none;
		width: 1.6rem;
		height: 1.6rem;
		border: 0;
		border-radius: 0.25rem;
	}
	.row.selected .preview {
		outline: none;
	}
	.row .preview img {
		object-fit: cover;
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
