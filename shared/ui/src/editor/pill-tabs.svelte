<script lang="ts" module>
	export type PillTabItem = { id: string; label: string; icon?: string; href?: string; title?: string; dim?: boolean; disabled?: boolean; testid?: string };
</script>

<script lang="ts">
	import PillTab from './pill-tab.svelte';

	/**
	 * Pill tabs, one of them on (deck-kit controls.rs `pills` / `pills_rounded`): side by side, --kit-seam apart, the one
	 * on pressed and rounded concentric with the pill around it. Links where an item has `href`, else `onselect`.
	 * `data-pill-tab` on each tab lets a caller measure them.
	 */
	let { items, on, onselect, small = false, label }: { items: readonly PillTabItem[]; on?: string | null; onselect?: (id: string) => void; small?: boolean; label?: string } = $props();
</script>

<span class="kit-pill-tabs" role="group" aria-label={label}>
	{#each items as item (item.id)}
		<PillTab label={item.label} icon={item.icon} href={item.href} title={item.title} dim={item.dim} disabled={item.disabled} {small} on={item.id === on} onclick={() => onselect?.(item.id)} data-pill-tab data-testid={item.testid} />
	{/each}
</span>

<style>
	.kit-pill-tabs {
		display: inline-flex;
		flex: 0 1 auto;
		align-items: center;
		gap: var(--kit-seam);
		min-width: 0;
		overflow: hidden;
		border-radius: var(--kit-r, var(--editor-radius));
	}
</style>
