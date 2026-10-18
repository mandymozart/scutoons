<script lang="ts">
	import { untrack } from 'svelte';
	import { COLLAGE_LOOKS, type CollagePiece, type PresetList } from '@scutoons/content';
	import { presetChoices, type PresetChoice } from '$lib/editor/collage-presets.ts';
	import { lookLabel } from '$lib/editor/collage-words.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LookPreview from '$lib/components/collage/look-preview.svelte';

	/**
	 * The piece's preset grid (research/COLLAGE.md §7, §12.4): As drawn, the ten code looks, then the custom presets of
	 * `colorations` in stored order (a deleted one only on the piece that names it: "Night (deleted)"). Each tile shows
	 * the selected piece's own file in that look (LookPreview – owner 2026-10-18) over its name; the chosen one inverted.
	 * More than GRID_PRESETS live custom presets: all of them move into the Custom fold under the grid (open on a piece
	 * that names one of them; its tiles are drawn only while it is open). A pick goes to `onpick` (none = As drawn) – also
	 * the chosen one again (the inspector takes the piece back to it when it has own values).
	 */
	let {
		piece,
		src,
		presets,
		onpick
	}: {
		piece: CollagePiece;
		/** the piece's file as a URL (the host's resolver) */
		src: string;
		/** the stored custom presets (presets-doc.ts `ownPresets`) */
		presets: PresetList;
		onpick: (look: string | undefined) => void;
	} = $props();

	const custom = $derived(presetChoices(presets, piece.preset));
	const chosen = $derived(custom.choices.find((c) => c.id === piece.preset));
	const customName = (c: PresetChoice) => (c.removed ? m['creator.preset_deleted_name']({ name: c.name }) : c.name);

	// the fold opens on a piece that names one of its presets – once per piece, and when the presets move into it (a
	// pick inside, an undo don't shut it)
	let open = $state(false);
	let openedFor: string | undefined;
	$effect.pre(() => {
		const at = `${piece.id} ${custom.inGrid}`;
		if (at === openedFor) return;
		openedFor = at;
		untrack(() => (open = !custom.inGrid && !!chosen));
	});
</script>

{#snippet tile(look: string | undefined, label: string, testid: string, removed = false)}
	<button type="button" class="tile" aria-pressed={piece.preset === look} title={label} onclick={() => onpick(look)} data-testid={testid}>
		<LookPreview {piece} {src} {look} {presets} />
		<span class="name" class:removed>{label}</span>
	</button>
{/snippet}

{#snippet customs()}
	{#each custom.choices as c (c.id)}{@render tile(c.id, customName(c), `piece-look-${c.id}`, c.removed)}{/each}
{/snippet}

<div class="looks">
	<div class="sc-seg is-grid tiles" role="group" aria-label={m['creator.piece_look']()} data-testid="piece-looks">
		{@render tile(undefined, m['creator.look_none'](), 'piece-look-none')}
		{#each COLLAGE_LOOKS as look (look.id)}{@render tile(look.id, lookLabel(look.id), `piece-look-${look.id}`)}{/each}
		{#if custom.inGrid}{@render customs()}{/if}
	</div>
	{#if !custom.inGrid}
		<details class="custom" bind:open data-testid="piece-looks-custom">
			<summary>{m['creator.preset_custom']()} ({custom.choices.length}){chosen && !open ? ` · ${customName(chosen)}` : ''}</summary>
			{#if open}
				<div class="sc-seg is-grid tiles" role="group" aria-label={m['creator.preset_custom']()}>{@render customs()}</div>
			{/if}
		</details>
	{/if}
</div>

<style>
	.looks {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	/* tiles of at least 4.5rem: four in the 22rem panel and on a 375px phone */
	.tiles {
		--seg-min: 4.5rem;
	}
	/* the name runs the tile's whole width ("Wie gezeichnet" in four columns), the preview sits inset (look-preview) */
	.tile {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: var(--space-025);
		min-width: 0;
		padding: var(--space-025) 0;
	}
	/* over the tile's transparent border too: "gezeichnet" fits four a row with the panel's scroll bar (75.6 of 78px) */
	.name {
		margin-inline: calc(-1 * var(--border-width-regular));
		display: -webkit-box;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		overflow: hidden;
		overflow-wrap: anywhere;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		text-align: center;
	}
	/* a deleted preset (only on the piece that names it): struck through, its name and "(deleted)" in the tooltip */
	.name.removed {
		text-decoration: line-through;
	}
	.custom[open] {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
</style>
