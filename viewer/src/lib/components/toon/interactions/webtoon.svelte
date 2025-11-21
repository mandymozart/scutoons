<script lang="ts">
	import type Group from '../group.svelte';

	let { group, onComplete }: { group: Group; onComplete: (jumpTo: string | null) => void } =
		$props();
	// void for the moment.
</script>
