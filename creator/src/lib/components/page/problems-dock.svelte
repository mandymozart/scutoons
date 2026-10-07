<script lang="ts">
	import type { ComponentProps } from 'svelte';
	import ProblemsPanel from '$lib/components/page/problems-panel.svelte';

	/**
	 * Problems in the shell's right dock as a panel that survives navigation (`kind: 'problems'`, agents/DOCKING.md
	 * "Panels across pages"): the chapter editor and the meta pages dock it, the next page's Problems take it over (its
	 * "notes" / "accepted" switches stay) and show that page's list.
	 */
	let props: ComponentProps<typeof ProblemsPanel> = $props();
</script>

<div class="problems-dock">
	<ProblemsPanel {...props} />
</div>

<style>
	.problems-dock {
		box-sizing: border-box;
		height: 100%;
		overflow: auto;
		padding: var(--space-1) var(--space-1) var(--space-3);
	}
</style>
