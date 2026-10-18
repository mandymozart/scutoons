<script lang="ts">
	import { customPreset, type CollagePiece, type Path } from '@scutoons/content';
	import { Icon } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { ownDiffers } from '$lib/editor/collage-presets.ts';
	import { livePreset, presetMenu, refreshPreset, savePreset } from '$lib/editor/preset-actions.ts';
	import { ownPresets } from '$lib/editor/presets-doc.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The line under the preset grid (research/COLLAGE.md §12.4): Save as preset … (the piece draws differently from what
	 * it names, or it names a deleted preset) · Update preset (it names a live custom preset and differs from it) · ••• –
	 * Rename … / Delete … that preset. Each control only where it applies, none disabled – most pieces show no line; none
	 * while the presets document loads or failed to load. The actions: preset-actions.ts.
	 */
	let { doc, path, piece, presets }: { doc: ContentDoc; path: Path; piece: CollagePiece; presets: ContentDoc } = $props();

	const own = $derived(ownPresets(presets));
	const live = $derived(livePreset(piece, presets));
	const differs = $derived(ownDiffers(piece, own));
	const save = $derived(differs || customPreset(piece.preset, own)?.removed === true);
	const host = $derived({ doc, path, presets });
</script>

{#if !presets.loading && !presets.error && (save || live)}
	<div class="preset-line" data-testid="preset-line">
		{#if save}<button type="button" class="secondary" onclick={() => savePreset(host, piece)} data-testid="preset-save">{m['creator.preset_save']()}</button>{/if}
		{#if live && differs}<button type="button" class="secondary" onclick={() => refreshPreset(host, piece)} data-testid="preset-update">{m['creator.preset_update']()}</button>{/if}
		{#if live}
			<button type="button" class="sc-round" title={live.name} aria-label={m['creator.preset_more']({ name: live.name })} onclick={() => presetMenu(host, piece)} data-testid="preset-more">
				<Icon name="more-horizontal" />
			</button>
		{/if}
	</div>
{/if}

<style>
	.preset-line {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05);
	}
</style>
