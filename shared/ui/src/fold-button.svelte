<script lang="ts">
	import Icon from './icon.svelte';

	/**
	 * The ONE fold button of a docked region (agents/DOCKING.md): round, Feather chevrons pointing to the side the region
	 * folds to – a left region (sidebar, script outline) folds left, the right dock right. Shown from 768px (phones have
	 * no rails). `title` carries the shortcut if there is one.
	 */
	let {
		side,
		label,
		title = label,
		controls,
		onclick,
		class: className = '',
		testid
	}: {
		side: 'left' | 'right' | 'bottom';
		label: string;
		title?: string;
		controls?: string;
		onclick: () => void;
		class?: string;
		testid?: string;
	} = $props();
</script>

<button type="button" class="sc-fold-button sc-round {className}" {onclick} aria-label={label} {title} aria-expanded="true" aria-controls={controls} data-testid={testid}>
	<Icon name={side === 'left' ? 'chevrons-left' : side === 'bottom' ? 'chevrons-down' : 'chevrons-right'} />
</button>

<style>
	@media (max-width: 767px) {
		.sc-fold-button {
			display: none;
		}
	}
</style>
