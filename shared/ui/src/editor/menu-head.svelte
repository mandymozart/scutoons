<script lang="ts">
	import type { Snippet } from 'svelte';

	/** A menu's title or a group's label in it (deck-kit controls.rs `checks`): small uppercase spaced, muted, a line high. */
	let { label, children }: { label?: string; children?: Snippet } = $props();
</script>

<span class="kit-menu-head">{#if children}{@render children()}{:else}{label}{/if}</span>

<style>
	.kit-menu-head {
		display: flex;
		flex: none;
		align-items: center;
		gap: var(--kit-icon-gap);
		min-height: var(--control-height);
		padding: 0 var(--editor-pad);
		color: var(--kit-menu-item-head-text);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
</style>
