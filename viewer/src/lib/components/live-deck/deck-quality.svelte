<script lang="ts">
	// The deck's render quality (#220, owner 2026-10-31: "a resolution selection, so that we can turn screws depending
	// on the deck's resources"): the level chosen – the highest this deck renders at – and Auto, the governor that steps
	// below it under load and back up with headroom (deck-space quality.rs; this only shows and forwards). The line
	// after the buttons is what the deck renders now: its level, the render scale, the frame time. The level names are
	// the stage HUD's (language-neutral, like a meter's).
	import type { QualityLevel, SpaceQuality } from '@scutoons/live-deck';
	import { m } from '$lib/paraglide/messages.js';

	let { quality, onquality }: { quality: SpaceQuality; onquality: (level: QualityLevel, auto: boolean) => void } = $props();
	const LEVELS: [QualityLevel, string][] = [
		['low', 'LOW'],
		['medium', 'MED'],
		['high', 'HIGH'],
		['ultra', 'ULTRA']
	];
</script>

<div class="quality" data-testid="deck-quality">
	<b>{m['live_deck.quality']()}</b>
	<button type="button" class="toggle" class:on={quality.auto} aria-pressed={quality.auto} title={m['live_deck.quality_auto_hint']()} onclick={() => onquality(quality.ceiling, !quality.auto)}>
		{m['live_deck.quality_auto']()}
	</button>
	<span class="seg" role="group" aria-label={m['live_deck.quality']()}>
		{#each LEVELS as [level, label] (level)}
			<button type="button" class:on={quality.ceiling === level} aria-pressed={quality.ceiling === level} title={m['live_deck.quality_level_hint']({ level: label })} onclick={() => onquality(level, quality.auto)}>
				{label}
			</button>
		{/each}
	</span>
	<span class="now" class:down={quality.level !== quality.ceiling}>{quality.level} {quality.scale.toFixed(2)}× {quality.frame ? `${quality.frame.toFixed(1)}MS` : ''}</span>
</div>

<style>
	.quality {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		min-height: var(--control-height);
		font-size: 0.625rem;
		text-transform: uppercase;
		white-space: nowrap;
	}
	b {
		font-weight: var(--font-weight-regular);
		color: color-mix(in srgb, var(--paper) 45%, transparent);
	}
	/* grouped and text controls: square (DESIGN "Round vs square"), at the control height – the panel's toggles */
	button {
		flex: none;
		height: var(--control-height);
		padding: 0 var(--space-05);
		border: var(--border-width-regular) solid color-mix(in srgb, var(--paper) 35%, transparent);
		border-radius: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-transform: uppercase;
		cursor: pointer;
	}
	.toggle {
		padding: 0 var(--space-075);
	}
	.seg {
		display: flex;
	}
	/* the borders overlap; the chosen one's lies on top */
	.seg button + button {
		margin-left: calc(-1 * var(--border-width-regular));
	}
	button.on {
		position: relative;
		border-color: var(--state-warning);
		color: var(--state-warning);
	}
	.now {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		color: color-mix(in srgb, var(--paper) 60%, transparent);
	}
	/* the governor stepped below the level chosen */
	.now.down {
		color: var(--state-warning);
	}
</style>
