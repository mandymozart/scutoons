<script lang="ts">
	import { rootPath, toonRelativePath } from '@scutoons/api';
	import { mediaText, mediaUrl } from './setup.ts';
	import AssetPickerSheet from './asset-picker-sheet.svelte';
	import { CloseIcon, Icon } from '@scutoons/ui';

	/**
	 * A media field that has no picture (audio, a ZIP download, a 3D model) as a slot like MediaSlot (UI rule: media fields are
	 * slots, not path text fields): the file's name is the button that opens the asset picker (that kind; `document` = a PDF, a score's MusicXML), an empty
	 * dashed slot when nothing is set, the path as the tooltip; × clears it. Audio plays in a small player under it.
	 * Paths are CDN-root paths (the catalog's) unless a `toon` is given: then toon-relative, like MediaSlot (chapter audio).
	 */
	let {
		toon = '',
		value,
		kind,
		label,
		place = '',
		onchange,
		testid,
		hideLabel = false,
		player = true,
		clearable = true
	}: {
		/** the toon folder ('' = paths from the CDN root, the catalog's) */
		toon?: string;
		value: string | null | undefined;
		kind: 'audio' | 'archive' | 'model' | 'document';
		label: string;
		place?: string;
		onchange: (path: string | null) => void;
		testid?: string;
		/** no visible label – the surrounding table / field shows it (the slot button keeps it as its accessible name) */
		hideLabel?: boolean;
		/** false: no player under the slot (the page has its own play button) */
		player?: boolean;
		/** false: no × (a required file whose row has its own remove) */
		clearable?: boolean;
	} = $props();

	const t = $derived(mediaText());
	let open = $state(false);
	const name = $derived(value ? value.split('/').pop() : '');
	const root = $derived(value ? rootPath(value, toon) : '');
</script>

<div class="file-slot" data-testid={testid}>
	{#if !hideLabel}<span class="sc-label">{label}</span>{/if}
	<div class="row">
		<button type="button" class="slot" class:empty={!value} onclick={() => (open = true)} title={value ? `${value}\n${t.chooseHint}` : t.chooseHint} aria-label="{label}: {value ?? t.choose}">
			{#if value}<Icon name={kind === 'audio' ? 'music' : kind === 'model' ? 'box' : kind === 'document' ? 'file-text' : 'archive'} /> <span class="name">{name}</span>{:else}<Icon name="plus" />{/if}
		</button>
		{#if value && clearable}
			<button type="button" class="sc-round" aria-label={t.remove} title={t.remove} onclick={() => onchange(null)}><CloseIcon /></button>
		{/if}
	</div>
	{#if value && kind === 'audio' && player}
		<audio controls preload="none" src={mediaUrl(root)}></audio>
	{/if}
	<AssetPickerSheet bind:open {toon} {kind} value={root} {place} title={label} onpick={(picked) => onchange(toonRelativePath(picked.path, toon))} />
</div>

<style>
	.file-slot {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		min-width: 0;
	}
	.row {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		min-width: 0;
	}
	.slot {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		box-sizing: border-box;
		min-width: 8rem;
		max-width: 100%;
		height: var(--control-height);
		padding: 0 var(--space-075);
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		cursor: pointer;
		overflow: hidden;
	}
	.slot.empty {
		justify-content: center;
		border-style: dashed;
		color: var(--muted);
	}
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	audio {
		width: 100%;
		max-width: 22rem;
	}
</style>
