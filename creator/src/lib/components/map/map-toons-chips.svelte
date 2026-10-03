<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';
	import { toggleToon } from '$lib/editor/map.ts';

	/** The toons a place / route belongs to: one toggle chip per toon (pressed = in it). */
	let { toons, value, onchange }: { toons: { id: string; name: string }[]; value: string[] | undefined; onchange: (next: string[]) => void } = $props();
</script>

<fieldset>
	<legend>{m['creator.map_toons']()}</legend>
	<div class="chips">
		{#each toons as t (t.id)}
			<button type="button" class="toggle" class:on={value?.includes(t.id)} aria-pressed={value?.includes(t.id)} onclick={() => onchange(toggleToon(value, t.id))}>{t.name}</button>
		{/each}
	</div>
</fieldset>

<style>
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
	.toggle {
		font: inherit;
		font-size: var(--type-caption-size);
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 0.45rem;
		border: var(--border-width-regular) solid var(--border-muted);
		background: var(--paper);
		color: var(--ink);
		cursor: pointer;
	}
	.toggle.on {
		border-color: var(--ink);
		background: var(--ink);
		color: var(--paper);
	}
</style>
