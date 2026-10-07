<script lang="ts">
	import { shellDock } from './dock.svelte.ts';

	/**
	 * Put on a page while it is still loading (a route's `{#await}` branch): a dock panel the last page left behind (the
	 * Flow, Problems, Info – `useDock` with a `kind`, agents/DOCKING.md "Panels across pages") stays held until this page
	 * has mounted and docked its own, so it is taken over instead of closing and opening again. Renders nothing.
	 */
	$effect(() => shellDock.hold());
</script>
