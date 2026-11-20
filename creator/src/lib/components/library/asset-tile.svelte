<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { formatDuration, type AssetFamily } from '@scutoons/media';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * One asset family as a tile (or a list row): preview, kind, use count (dashed 0 = unused), duration, ⧉ n = its
	 * versions (missing ones counted apart); hover Use · Replace · ⋯. Click selects, double-click uses, drag hands the
	 * file to a stage (`application/x-scutoons-asset`).
	 */
	let {
		family,
		selected = false,
		dimmed = false,
		row = false,
		trayOpen = false,
		onselect,
		onuse,
		onreplace,
		ondetails,
		ontray
	}: {
		family: AssetFamily;
		selected?: boolean;
		/** a pick that this family does not fit: shown, not usable */
		dimmed?: boolean;
		row?: boolean;
		trayOpen?: boolean;
		onselect: () => void;
		onuse: () => void;
		onreplace: (file: File) => void;
		ondetails: () => void;
		ontray?: () => void;
	} = $props();

	const ICONS: Record<AssetFamily['kind'], string> = { image: 'image', video: 'film', audio: 'music', model: 'box', other: 'file' };
	let fileInput: HTMLInputElement | null = $state(null);
	const duration = $derived(formatDuration(family.duration));

	function drag(event: DragEvent) {
		if (!event.dataTransfer || dimmed) return event.preventDefault();
		event.dataTransfer.effectAllowed = 'copy';
		event.dataTransfer.setData('application/x-scutoons-asset', JSON.stringify({ path: family.main, key: family.key, kind: family.kind }));
		event.dataTransfer.setData('text/plain', family.main);
	}

	function key(event: KeyboardEvent) {
		if (event.key === 'Enter') onuse();
		else if (event.key === ' ') (event.preventDefault(), onselect());
	}
</script>

<div
	class="tile"
	class:row
	class:selected
	class:dimmed
	role="option"
	tabindex="0"
	aria-selected={selected}
	aria-disabled={dimmed}
	title={family.key}
	draggable={!dimmed}
	data-control-exempt="asset tile"
	data-testid="library-tile"
	data-key={family.key}
	onclick={onselect}
	ondblclick={() => !dimmed && onuse()}
	onkeydown={key}
	ondragstart={drag}
>
	<span class="preview">
		{#if family.preview}
			<img src={asset(family.preview)} alt="" loading="lazy" draggable="false" />
		{:else}
			<span class="glyph"><Icon name={ICONS[family.kind]} size={row ? '1rem' : '1.6rem'} /></span>
		{/if}
	</span>
	{#if row}
		<span class="name">{family.title}</span>
		<span class="meta">{m[`library.kind_${family.kind}`]()}</span>
		{#if duration}<span class="meta mono">{duration}</span>{/if}
		<span class="uses mono" class:zero={family.usages === 0} title={m['library.uses']({ count: family.usages })}>{family.usages}</span>
	{:else}
		<span class="kind">{m[`library.kind_${family.kind}`]()}</span>
		<span class="uses mono" class:zero={family.usages === 0} title={m['library.uses']({ count: family.usages })}>{family.usages}</span>
		{#if duration}<span class="duration mono">{duration}</span>{/if}
		<span class="name">{family.title}</span>
	{/if}
	{#if family.versions.length > 1 || family.missing.length}
		<button
			type="button"
			class="versions mono"
			class:open={trayOpen}
			class:warn={family.missing.length > 0}
			data-control-exempt="tile badge"
			title={m['library.versions_title']({ count: family.versions.length, missing: family.missing.length })}
			onclick={(e) => (e.stopPropagation(), ontray ? ontray() : ondetails())}
			data-testid="tile-versions"><Icon name="copy" size="0.75rem" /> {family.versions.length}{#if family.missing.length}<span class="miss">+{family.missing.length}</span>{/if}</button
		>
	{/if}
	{#if !dimmed}
		<span class="actions">
			<button type="button" class="act use" data-control-exempt="tile action" onclick={(e) => (e.stopPropagation(), onuse())} data-testid="tile-use">{m['library.use']()}</button>
			<button type="button" class="act round" data-control-exempt="tile action" title={m['library.replace']()} aria-label={m['library.replace']()} onclick={(e) => (e.stopPropagation(), fileInput?.click())}><Icon name="repeat" size="0.85rem" /></button>
			<button type="button" class="act round" data-control-exempt="tile action" title={m['library.details']()} aria-label={m['library.details']()} onclick={(e) => (e.stopPropagation(), ondetails())}><Icon name="more-horizontal" size="0.85rem" /></button>
		</span>
	{/if}
	<input bind:this={fileInput} type="file" hidden onchange={(e) => { const f = e.currentTarget.files?.[0]; if (f) onreplace(f); e.currentTarget.value = ''; }} />
</div>

<style>
	.tile {
		position: relative;
		aspect-ratio: 1;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: var(--editor-radius);
		background: var(--editor-stage);
		overflow: hidden;
		cursor: pointer;
		outline: none;
		font-size: var(--editor-small);
	}
	.tile:focus-visible,
	.tile.selected {
		outline: 0.125rem solid var(--editor-accent);
		outline-offset: 0.0625rem;
	}
	.tile.dimmed {
		opacity: 0.35;
		cursor: default;
	}
	.preview {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
	}
	.preview img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
	.glyph {
		color: var(--editor-muted);
	}
	.kind,
	.uses,
	.duration,
	.versions {
		position: absolute;
		padding: 0 0.3rem;
		border-radius: 0.3rem;
		background: var(--editor-surface);
		color: var(--editor-muted);
		font-size: var(--editor-label);
		line-height: 1.5;
	}
	.kind {
		left: 0.3rem;
		top: 0.3rem;
	}
	.uses {
		right: 0.3rem;
		top: 0.3rem;
		color: var(--editor-text);
		border: var(--editor-edge) solid transparent;
	}
	.uses.zero {
		border-style: dashed;
		border-color: var(--editor-muted);
		color: var(--editor-muted);
	}
	.duration {
		left: 0.3rem;
		bottom: 1.7rem;
	}
	.versions {
		right: 0.3rem;
		bottom: 1.7rem;
		display: inline-flex;
		align-items: center;
		gap: 0.2rem;
		border: var(--editor-edge) solid transparent;
		cursor: pointer;
	}
	.versions.open {
		background: var(--editor-accent);
		color: var(--editor-primary-ink);
	}
	.versions .miss {
		color: var(--editor-warn);
	}
	.versions.open .miss {
		color: inherit;
	}
	.name {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		padding: 0.9rem 0.4rem 0.25rem;
		background: linear-gradient(transparent, var(--editor-surface));
		color: var(--editor-text);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.actions {
		position: absolute;
		left: 0;
		right: 0;
		top: 50%;
		transform: translateY(-50%);
		display: none;
		justify-content: center;
		gap: 0.3rem;
	}
	.tile:hover .actions,
	.tile:focus-within .actions {
		display: flex;
	}
	.act {
		height: 1.6rem;
		border: var(--editor-edge) solid var(--editor-line);
		background: var(--editor-surface);
		color: var(--editor-text);
		cursor: pointer;
	}
	.actions .use {
		padding: 0 0.6rem;
		border-radius: 1rem;
		border-color: var(--editor-primary-line);
		background: var(--editor-primary);
		color: var(--editor-primary-ink);
	}
	.actions .round {
		width: 1.6rem;
		padding: 0;
		border-radius: 50%;
		display: inline-grid;
		place-items: center;
	}
	.mono {
		font-family: var(--editor-mono, ui-monospace, monospace);
	}

	/* list rows */
	.tile.row {
		aspect-ratio: auto;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-height: var(--editor-tree-row);
		padding: 0 0.4rem 0 0.2rem;
		border-color: transparent;
		background: none;
	}
	.tile.row:hover {
		background: var(--editor-hover);
	}
	.row .preview {
		position: static;
		flex: none;
		width: 1.6rem;
		height: 1.6rem;
		border-radius: 0.25rem;
		overflow: hidden;
		background: var(--editor-stage);
	}
	.row .preview img {
		object-fit: cover;
	}
	.row .name {
		position: static;
		flex: 1 1 auto;
		min-width: 0;
		padding: 0;
		background: none;
	}
	.row .meta {
		flex: none;
		color: var(--editor-muted);
		font-size: var(--editor-label);
	}
	.row .uses,
	.row .versions {
		position: static;
		flex: none;
	}
	.row .actions {
		position: static;
		transform: none;
		flex: none;
	}
</style>
