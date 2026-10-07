<script lang="ts">
	import type { DockContent } from './dock.svelte.ts';

	/**
	 * A docked panel the shell mounts itself (`useDock` with `kind` + `content`, agents/DOCKING.md "Panels across pages"):
	 * the same component stays mounted when the next page takes the panel over – it only gets that page's props.
	 */
	let { content }: { content: DockContent } = $props();
	const Panel = $derived(content.component);
	const input = $derived(content.props());
</script>

<Panel {...input} />
