<script lang="ts">
	import type { Snippet } from 'svelte';
	import { setDockSlot } from './dock.svelte.ts';

	/**
	 * Around each docked panel in the AppShell (agents/DOCKING.md "Several panels"): tells the panel's `DockFold` which of
	 * the open panels it folds / maximizes. Renders only its children.
	 */
	let { id, children }: { id: number; children: Snippet } = $props();
	setDockSlot(() => id);
</script>

{@render children()}
