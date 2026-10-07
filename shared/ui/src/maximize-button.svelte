<script lang="ts">
	import Icon from './icon.svelte';
	import { shellDock, type MaximizeTarget } from './dock.svelte.ts';

	/**
	 * Maximizes a region for a moment (owner 2026-10-06, agents/DOCKING.md "Maximize"): round, Feather `maximize-2`
	 * ("Maximize"), while maximized `minimize-2` ("Restore"). `target`: `'dock'` (the most recently used docked panel) or `dock:<id>` (that panel – in its header next to
	 * its ✕, rendered by `DockFold`) or a main-area region's id (the slice inspector). Ctrl + Space, Esc and the button
	 * restore. Hidden on phones (the dock covers the page there anyway).
	 */
	let {
		target = 'dock',
		label = 'Maximize',
		restoreLabel = 'Restore',
		controls,
		testid = 'maximize'
	}: { target?: MaximizeTarget; label?: string; restoreLabel?: string; controls?: string; testid?: string } = $props();

	const on = $derived(target === 'dock' ? !!shellDock.maximizedPanel : shellDock.maximized === target);
	const text = $derived(on ? restoreLabel : label);
</script>

<button
	type="button"
	class="sc-maximize-button sc-round"
	onclick={() => shellDock.maximize(on ? null : target)}
	aria-label={text}
	title="{text} (Ctrl + Space)"
	aria-pressed={on}
	aria-controls={controls}
	data-testid={testid}
>
	<Icon name={on ? 'minimize-2' : 'maximize-2'} />
</button>

<style>
	@media (max-width: 767px) {
		.sc-maximize-button {
			display: none;
		}
	}
</style>
